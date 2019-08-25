use super::handle::ConnectionHandle;
use super::ws::WebSocketWriter;
use super::FernspielEvent;

use super::cause::ShutdownCause;
use crate::result::Result;

use crossbeam_channel::{bounded, select, Receiver, Sender, TrySendError};
use log::{debug, error, trace};
use websocket::OwnedMessage;

use std::thread::spawn;

pub type ConnectResult = std::result::Result<(), TrySendError<(ConnectionHandle, WebSocketWriter)>>;
pub type UnicastResult = std::result::Result<(), TrySendError<(ConnectionHandle, OwnedMessage)>>;
pub type BroadcastResult = std::result::Result<(), TrySendError<OwnedMessage>>;

const MSG_QUEUE_SIZE: usize = 256;

#[derive(Clone)]
pub struct Relay {
    new_connections: Sender<(ConnectionHandle, WebSocketWriter)>,
    messages: Sender<(Address, OwnedMessage)>,
}

impl Relay {
    pub fn spawn(events: Receiver<FernspielEvent>) -> Self {
        let (conn_tx, msg_tx) = RelayWorker::spawn(events);
        Self {
            new_connections: conn_tx,
            messages: msg_tx,
        }
    }

    pub fn connect(&self, handle: ConnectionHandle, connection: WebSocketWriter) -> ConnectResult {
        match self.new_connections.try_send((handle, connection)) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full((handle, writer))) => Err(TrySendError::Full((handle, writer))),
            Err(TrySendError::Disconnected((handle, writer))) => {
                Err(TrySendError::Disconnected((handle, writer)))
            }
        }
    }

    pub fn unicast(&self, address: ConnectionHandle, msg: OwnedMessage) -> UnicastResult {
        trace!("sending message: \"{:?}\" to {:?}", &msg, &address);
        match self.messages.try_send((Address::Unicast(address), msg)) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full((_, msg))) => Err(TrySendError::Full((address, msg))),
            Err(TrySendError::Disconnected((_, msg))) => {
                Err(TrySendError::Disconnected((address, msg)))
            }
        }
    }

    #[allow(dead_code)]
    pub fn broadcast(&self, msg: OwnedMessage) -> BroadcastResult {
        trace!("broadcasting message: {:?}", &msg);
        match self.messages.try_send((Address::Broadcast, msg)) {
            Ok(_) => Ok(()),
            Err(TrySendError::Full((_, msg))) => Err(TrySendError::Full(msg)),
            Err(TrySendError::Disconnected((_, msg))) => Err(TrySendError::Disconnected(msg)),
        }
    }
}

enum Address {
    #[allow(dead_code)]
    Broadcast,
    Unicast(ConnectionHandle),
}

struct RelayWorker {
    new_connections: Receiver<(ConnectionHandle, WebSocketWriter)>,
    connections: Vec<(ConnectionHandle, WebSocketWriter)>,
    messages: Receiver<(Address, OwnedMessage)>,
    events: Receiver<FernspielEvent>,
}

impl RelayWorker {
    pub fn spawn(
        events: Receiver<FernspielEvent>,
    ) -> (
        Sender<(ConnectionHandle, WebSocketWriter)>,
        Sender<(Address, OwnedMessage)>,
    ) {
        let (conn_tx, conn_rx) = bounded(MSG_QUEUE_SIZE);
        let (msg_tx, msg_rx) = bounded(MSG_QUEUE_SIZE);
        spawn(move || Self::new(conn_rx, msg_rx, events).run());
        (conn_tx, msg_tx)
    }

    fn new(
        new_connections: Receiver<(ConnectionHandle, WebSocketWriter)>,
        messages: Receiver<(Address, OwnedMessage)>,
        events: Receiver<FernspielEvent>,
    ) -> Self {
        Self {
            new_connections,
            messages,
            events,
            connections: vec![],
        }
    }

    fn run(&mut self) {
        // run until error is returned when remote end hung up
        while let Ok(_) = self.recv() {}
    }

    fn recv(&mut self) -> Result<()> {
        select! {
            // return with error when remote end hung up
            recv(self.new_connections) -> connection => self.connections.push(connection?),
            recv(self.messages) -> msg => match msg? {
                (Address::Broadcast, ref msg) => self.broadcast_message(msg),
                (Address::Unicast(handle), ref msg) => self.unicast_message(handle, msg),
            },
            recv(self.events) -> evt => self.broadcast_event(evt?)
        }
        Ok(())
    }

    fn broadcast_event(&mut self, evt: FernspielEvent) {
        serde_yaml::to_string(&evt)
            .map(OwnedMessage::Text)
            .map(|msg| self.broadcast_message(&msg))
            .unwrap_or_else(|e| {
                error!("failed to broadcast event: {}", e);
            })
    }

    fn broadcast_message(&mut self, msg: &OwnedMessage) {
        trace!("broadcasting message {:?}", msg);
        self.connections
            .drain_filter(|(h, c)| !Self::try_send(*h, c, msg))
            .for_each(|(_, dropped)| Self::shutdown(dropped));
    }

    fn unicast_message(&mut self, handle: ConnectionHandle, msg: &OwnedMessage) {
        let addressee_idx = self
            .connections
            .iter_mut()
            .position(|(conn_handle, _)| *conn_handle == handle);

        if let Some(addressee_idx) = addressee_idx {
            let ok = {
                let (handle, ref mut connection) = &mut self.connections[addressee_idx];
                Self::try_send(*handle, connection, msg)
            };
            if !ok {
                let (_, conn) = self.connections.swap_remove(addressee_idx);
                Self::shutdown(conn);
            }
        }
    }

    fn try_send(handle: ConnectionHandle, conn: &mut WebSocketWriter, msg: &OwnedMessage) -> bool {
        trace!("sending message {:?} to {:?}", msg, handle);
        let is_close = msg.is_close();
        match conn.send_message(msg) {
            Ok(_) => {
                // sending worked, keep the connection, unless this is a close message
                !is_close
            }
            Err(err) => {
                error!("fernspielctl sending error: {}", err);
                true // remove connection from vec and pass on
            }
        }
    }

    fn shutdown(connection: WebSocketWriter) {
        connection
            .shutdown() // shut down the writer end of failed ones
            .unwrap_or_else(|e| {
                debug!(
                    "failed to orderly shutdown connection after sending error: {}",
                    e
                )
            });
    }
}

impl Drop for RelayWorker {
    fn drop(&mut self) {
        // send close message,
        // this will shut down the writing half of connections
        // and clear the connections vector
        self.broadcast_message(&ShutdownCause::Done.into_close_msg());
    }
}
