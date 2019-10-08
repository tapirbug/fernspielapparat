use super::decoder::Decoder;
use super::handle::{ConnectionHandle, ConnectionHandleGenerator};
use super::relay::Relay;
use super::ws::{WebSocketClient, WebSocketServer, WebSocketUpgrade};

use crate::result::Result;
use crate::serve::{FernspielEvent, Request};

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError, select};
use failure::{bail, format_err};
use log::{debug, error, info, trace};
use std::thread::spawn;

const WS_PROTOCOL: &str = "fernspielctl";

/// Receives requests from websocket connections, negotiates the `fernspielctl`
/// protocol, and registers the new connections with the relay. A decoder thread
/// is launched for each new connection that decodes incoming requests and compiles
/// phonebooks.
pub struct Acceptor {
    channel: Sender<Request>,
    relay: Relay,
    handle_gen: ConnectionHandleGenerator,
    shutdown_signal: Receiver<()>,
}

impl Acceptor {
    /// Spawns a worker and returns a sender that triggers shutdown.
    pub fn spawn(
        on_hostname_and_port: &str,
        sender: Sender<Request>,
        receiver: Receiver<FernspielEvent>,
    ) -> Result<Sender<()>> {
        let server = WebSocketServer::bind(on_hostname_and_port)?;
        let (shutdown_tx, shutdown_rx) = bounded(1);

        spawn(move || {
            Self {
                channel: sender,
                relay: Relay::spawn(receiver),
                handle_gen: ConnectionHandle::generate(),
                shutdown_signal: shutdown_rx,
            }
            .run(server)
        });

        Ok(shutdown_tx)
    }

    /// Keeps the acceptor running until the shutdown signal
    /// is received.
    fn run(&mut self, mut ws: WebSocketServer) {
        let (accept_tx, accept_rx) = bounded(4);

        spawn(move || {
            loop {
                if let Ok(request) = ws.accept() {
                    if let Err(_) = accept_tx.send(request) {
                        break;
                    }
                }
            }
        });

        // run until shutdown signal received
        loop {
            select! {
                // return with error when remote end hung up
                recv(accept_rx) -> connection => {
                    match connection {
                        Ok(conn) => {
                            if let Err(err) = accept(conn).and_then(|c| self.communicate(c)) {
                                error!("could not accept connection {:?}", err);
                            }
                        },
                        Err(e) =>  {
                            debug!("accept recv error {:?}", e);
                            break
                        }
                    }
                },
                recv(self.shutdown_signal) -> _ => break
            }
        }

        trace!("shutting down connection acceptor")
    }

    /// Loops through incoming messages from the client and handles
    /// them.
    fn communicate(&mut self, client: WebSocketClient) -> Result<()> {
        if let Err(e) = client.set_nonblocking(false) {
            error!("failed to make blocking websocket connection pair: {}", e);
        }

        let (receiver, sender) = client.split()?;
        let handle = self.handle_gen.next().ok_or_else(|| {
            format_err!(
                "Too many connections or running for too long, \
                 encountered handle overflow, shutting down server",
            )
        })?;

        match self.relay.connect(handle, sender) {
            Ok(()) => (),
            Err(TrySendError::Disconnected((_, sender))) => {
                sender.shutdown_all().unwrap_or_else(|e| {
                    debug!(
                        "Failed to terminate connection while server is shutting down: {}",
                        e
                    )
                });
                bail!("Relay hung up, exiting server") // exit server
            }
            Err(TrySendError::Full((_, sender))) => {
                sender.shutdown_all().unwrap_or_else(|e| {
                    debug!("Failed to terminate connection during overload: {}", e)
                });
                error!("Too many connections, rejecting incoming connection");
                // do not bail, continue waiting for new connections
            }
        }

        Decoder::spawn(handle, receiver, &self.relay, self.channel.clone());

        Ok(())
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

        let ip = client
            .peer_addr()
            .map_err(|e| format_err!("address of peer could not be detected, error: {:?}", e))?;

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
