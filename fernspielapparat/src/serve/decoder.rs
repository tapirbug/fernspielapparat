use crate::result::Result;

use super::cause::ShutdownCause;
use super::handle::ConnectionHandle;
use super::relay::Relay;
use super::ws::WebSocketReader;
use super::Request;

use crossbeam_channel::Sender;
use failure::format_err;
use log::{debug, trace};
use websocket::OwnedMessage;

use std::thread::spawn;

pub struct Decoder {
    handle: ConnectionHandle,
    relay: Relay,
    channel: Sender<Request>,
}

impl Decoder {
    pub fn spawn(
        handle: ConnectionHandle,
        connection: WebSocketReader,
        relay: &Relay,
        request_channel: Sender<Request>,
    ) {
        let mut decoder = Decoder {
            handle,
            relay: relay.clone(),
            channel: request_channel,
        };
        spawn(move || match decoder.receive(connection) {
            Ok(()) => debug!("decoder exiting after successful operation"),
            Err(err) => debug!("decoder exiting after error {:?}", err),
        });
    }

    fn receive(&mut self, mut connection: WebSocketReader) -> Result<()> {
        for message in connection.incoming_messages() {
            // shut down worker on I/O errors
            if let Some(shutdown_cause) = self.handle(message?)? {
                match shutdown_cause {
                    ShutdownCause::Done => {
                        // shut down when close requested from client
                        break;
                    }
                }
            }
        }

        connection
            .shutdown()
            .unwrap_or_else(|e| debug!("failed to shut down receiving end of connection: {}", e));

        Ok(())
    }

    fn handle(&mut self, message: OwnedMessage) -> Result<Option<ShutdownCause>> {
        match message {
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
                self.handle_request(text)?; // abort on invalid messages
                Ok(None)
            }
            // websocket-specified pong message should only be sent in response
            // to ping messages, which this application never sends, ignore if
            // receiving a pong anyway
            OwnedMessage::Pong(_) => Ok(None),
            // client pings us, respond with same payload and wait for next message
            OwnedMessage::Ping(ping) => {
                if let Err(err) = self.relay.unicast(self.handle, OwnedMessage::Pong(ping)) {
                    debug!("failed to enqueue pong message: {}", err)
                }
                Ok(None)
            }
            // the protocol does not define any binary messages, panic if one
            // is received
            OwnedMessage::Binary(_) => {
                debug!("received binary message, but only text is supported, discarding message");
                Ok(None)
            }
            // client requested to shut down the connection
            OwnedMessage::Close(_) => {
                debug!("orderly closing websocket connection after shutdown request from client");
                Ok(Some(ShutdownCause::Done))
            }
        }
    }

    fn handle_request(&mut self, request: String) -> Result<()> {
        match Request::decode(request) {
            Err(err) => {
                debug!("received invalid request {}", err);
                // TODO send error back
                Ok(())
            }
            Ok(request) => self
                .channel
                .send(request)
                .map_err(|e| format_err!("request received but server is shutting down: {:?}", e)),
        }
    }
}
