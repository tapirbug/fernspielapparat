use super::acceptor::Acceptor;
use super::{FernspielEvent, Request};

use crate::result::Result;

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use log::{error, trace};

pub struct Server {
    events: Sender<FernspielEvent>,
    signal_shutdown: Sender<()>,
    shutdown: bool,
    invocations: Receiver<Request>,
}

/// A websocket server running in the background and listening for
/// requests from a controlling application, e.g. the `fernspieleditor`
/// webapp.
impl Server {
    /// Maximum unhandled messages in queue before incoming requests
    /// are dropped without handling them.
    const MSG_QUEUE_SIZE: usize = 64;

    /// Spins up a background server on the given hostname
    /// and port. Client code needs to regularly poll for
    /// requests and can publish events through the server.
    pub fn spawn(on_hostname_and_port: &str) -> Result<Server> {
        let (invoke_tx, invoke_rx) = bounded(Self::MSG_QUEUE_SIZE);
        let (event_tx, event_rx) = bounded(Self::MSG_QUEUE_SIZE);

        let signal_shutdown = Acceptor::spawn(on_hostname_and_port, invoke_tx, event_rx)?;

        Ok(Server {
            events: event_tx,
            invocations: invoke_rx,
            signal_shutdown,
            shutdown: false,
        })
    }

    /// Terminates the background thread, cannot be undone.
    pub fn shutdown(&mut self) {
        if !self.shutdown {
            self.shutdown = true;
            self.signal_shutdown
                .try_send(())
                .unwrap_or_else(|e| error!("failed to shut down fernspielctl server: {}", e))
        }
    }

    /// Tries to get the next request from the server, if any.
    pub fn poll(&self) -> Option<Request> {
        if self.shutdown {
            return None;
        }

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

    /// Publishes the given event to all connected clients.
    pub fn publish(&self, evt: FernspielEvent) {
        trace!("publishing event {:?}", evt);
        if !self.shutdown {
            self.events
                .try_send(evt)
                .unwrap_or_else(|error| error!("failed to publish event: {}", error));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown()
    }
}
