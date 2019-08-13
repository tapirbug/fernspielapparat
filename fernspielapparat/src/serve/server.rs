use super::{FernspielEvent, Request};
use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use failure::Error;
use log::error;

type Result<T> = std::result::Result<T, Error>;

pub struct Server {
    events: Sender<FernspielEvent>,
    invocations: Receiver<Request>,
}

/// A websocket server running in the background and listening for
/// requests from a controlling application, e.g. the `fernspieleditor`
/// webapp.
impl Server {
    /// Maximum unhandled messages in queue before blocking upon receiving
    /// new requests.
    const MSG_QUEUE_SIZE: usize = 64;

    pub fn spawn(on_hostname_and_port: &str) -> Result<Server> {
        let (invoke_tx, invoke_rx) = bounded(Self::MSG_QUEUE_SIZE);
        let (event_tx, event_rx) = bounded(Self::MSG_QUEUE_SIZE);
        worker::Worker::spawn(on_hostname_and_port, invoke_tx, event_rx)?;
        // TODO spawn worker for publish protocol
        Ok(Server {
            invocations: invoke_rx,
            events: event_tx,
        })
    }

    pub fn poll(&self) -> Option<Request> {
        match self.invocations.try_recv() {
            Ok(req) => Some(req),
            Err(TryRecvError::Empty) => None,
            Err(error) => {
                error!(
                    "failed to check disconnected server worker for new requests: {}",
                    error
                );
                None
            }
        }
    }

    pub fn publish(&self, evt: FernspielEvent) {
        self.events.send(evt).unwrap_or_else(|error| {
            error!(
                "failed to publish event because server worker has shut down: {}",
                error
            )
        });
    }
}

mod worker {
    use crate::serve::FernspielEvent;
    use crate::serve::Request;

    use crossbeam_channel::{bounded, select, Receiver, Sender};
    use failure::{bail, format_err, Error};
    use log::{debug, error, info, trace};
    use websocket;
    use websocket::message::{CloseData, OwnedMessage};

    use std::thread::spawn;

    type Result<T> = std::result::Result<T, Error>;
    type WebSocketServer = websocket::sync::Server<websocket::server::NoTlsAcceptor>;
    type WebSocketUpgrade = websocket::server::upgrade::WsUpgrade<
        std::net::TcpStream,
        Option<websocket::server::upgrade::sync::Buffer>,
    >;
    type WebSocketClient = websocket::sync::Client<std::net::TcpStream>;

    const WS_PROTOCOL: &str = "fernspielctl";

    enum ShutdownCause {
        Done,
        UnsupportedMsg,
    }

    impl ShutdownCause {
        const UNSUPPORTED_MESSAGE_CODE: u16 = 1;
        const UNSUPPORTED_MESSAGE_REASON_PHRASE: &'static str =
            "connection aborted after receiving a corrupt or unsupported message";

        fn into_close_msg(self) -> OwnedMessage {
            OwnedMessage::Close(match self {
                ShutdownCause::Done => None,
                ShutdownCause::UnsupportedMsg => Some(CloseData::new(
                    Self::UNSUPPORTED_MESSAGE_CODE,
                    Self::UNSUPPORTED_MESSAGE_REASON_PHRASE.to_string(),
                )),
            })
        }
    }

    /// Receives requests from websocket connections and relays them
    /// to the server stored on the receiver thread for consumption
    /// by client code.
    pub struct Worker {
        channel: Sender<Request>,
        events: Receiver<FernspielEvent>,
    }

    impl Worker {
        pub fn spawn(
            on_hostname_and_port: &str,
            sender: Sender<Request>,
            receiver: Receiver<FernspielEvent>,
        ) -> Result<()> {
            let server = WebSocketServer::bind(on_hostname_and_port)?;

            spawn(move || {
                Worker {
                    channel: sender,
                    events: receiver,
                }
                .run(server)
            });

            Ok(())
        }

        fn run(&mut self, ws: WebSocketServer) {
            let reqs = ws
                // drop failed connection attempts
                .filter_map(std::result::Result::ok);

            for request in reqs {
                summarize_session(accept(request).and_then(|c| self.communicate(c)));
            }
        }

        /// Loops through incoming messages from the client and handles
        /// them.
        fn communicate(&mut self, client: WebSocketClient) -> Result<()> {
            let (mut receiver, mut sender) = client.split()?;

            // spawn separate thread for sending that is used both by
            // the server handle for publishing as well as for the worker
            // impl for control messages
            let events = self.events.clone();
            let (control_tx, control_rx) = bounded::<OwnedMessage>(12);
            spawn(move || {
                let shutdown_cause = loop {
                    let msg = select!(
                        recv(control_rx) -> msg => match msg {
                            Err(_) => break Ok(ShutdownCause::Done), // exit, remote end hung up
                            // control message like pong or shutdown
                            Ok(control) => if control.is_close() {
                                break Ok(ShutdownCause::Done);
                            } else {
                                control
                            },
                        },
                        recv(events) -> evt => match evt {
                            Err(_) => break Ok(ShutdownCause::Done), // exit, remote end hung up
                            Ok(evt) => serde_yaml::to_string(&evt)
                                .map(OwnedMessage::Text)
                                .unwrap_or_else(|e| {
                                    error!("failed to serialize event, sending ping instead: {}", e);
                                    OwnedMessage::Ping(Vec::new())
                                })
                        }
                    );

                    if let Err(e) = sender.send_message(&msg) {
                        error!(
                            "failed to send websockets event, clsoing connection after error: {}",
                            e
                        );
                        break Err(e);
                    }
                };

                if let Ok(shutdown_cause) = shutdown_cause {
                    // only try to orderly shutdown when did not exit due to error
                    sender
                        .send_message(&shutdown_cause.into_close_msg())
                        .unwrap_or_else(|e| {
                            error!("failed to send shutdown message, error: {}", e);
                        });
                }

                sender.shutdown_all().unwrap_or_else(|e| {
                    error!("failed to shut down websocket connection, error: {}", e);
                }); // shut down reader as well in one go
            });

            // and use this thread for reading of remote invocations
            for message in receiver.incoming_messages() {
                match message? {
                    // got text message, handle and wait for next message
                    OwnedMessage::Text(text) => {
                        trace!(
                            "fernspielctl message received: {msg}",
                            msg = {
                                let mut summary = text.clone();
                                // do not spam the trace log too much, cut off message at some point
                                summary.truncate(256);
                                summary
                            }
                        );
                        // abort connection on garbage messages
                        match self.handle(text) {
                            Ok(()) => (),
                            Err(e) => {
                                control_tx.send(ShutdownCause::UnsupportedMsg.into_close_msg())
                                    .unwrap_or_else(|e| error!("failed to enequeue shutdown request after invalid message: {}", e));
                                return Err(e);
                            }
                        }
                    }
                    // websocket-specified pong message should only be sent in response
                    // to ping messages, which this application never sends, ignore if
                    // receiving a pong anyway
                    OwnedMessage::Pong(_) => (),
                    // the protocol does not define any binary messages, panic if one
                    // is received
                    OwnedMessage::Binary(_) => bail!(
                        "received binary message, but only text is supported, connection aborted"
                    ),
                    // client requested to shut down the connection
                    OwnedMessage::Close(_) => {
                        debug!("orderly closing websocket connection after shutdown request from client");
                        break; // writer thread will shut down the reader too when this function exits
                    }
                    // client pings us, respond with same payload and wait for next message
                    OwnedMessage::Ping(ping) => {
                        control_tx
                            .send(OwnedMessage::Pong(ping))
                            .unwrap_or_else(|e| error!("failed to enqueue pong message: {}", e));
                    }
                }
            }

            // exit drops the channel to the write worker, which closes it, shutting down websocket connection
            Ok(())
        }

        fn handle(&mut self, request: String) -> Result<()> {
            let request = Request::decode(request)?;
            self.channel
                .send(request)
                .map_err(|e| format_err!("request received but server is shutting down: {:?}", e))
        }
    }

    /// rejects or accepts the given request, sets the protocol
    /// and returns the client on success.
    ///
    /// Returns an error when protocol negotiation failed.
    ///
    /// New connections are logged with info level.
    fn accept(request: WebSocketUpgrade) -> Result<WebSocketClient> {
        if request.protocols().contains(&WS_PROTOCOL.to_string()) {
            let client = request
                .use_protocol(WS_PROTOCOL)
                .accept()
                .map_err(|(_, e)| {
                    format_err!("could not initialize websocket connection, error: {:?}", e)
                })?;

            let ip = client.peer_addr().map_err(|e| {
                format_err!("address of peer could not be detected, error: {:?}", e)
            })?;

            info!("fernspielctl client connected: {}", ip);
            Ok(client)
        } else {
            request.reject()
                .map_err(|(_, e)| format_err!(
                    "aborting rejection of websocket connection attempt, rejection failure reason: {:?}",
                    e
                ))?;

            bail!("fernspielctl protocol unsupported by websocket connection")
        }
    }

    fn summarize_session(result: Result<()>) {
        match result {
            Ok(_) => {
                debug!("fernspielctl connection shut down orderly, waiting for new connection.")
            }
            Err(err) => error!(
                "fernspielctl connection unexpectedly aborted, error: {}",
                err
            ),
        }
    }
}
