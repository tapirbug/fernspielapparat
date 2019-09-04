use websocket::OwnedMessage;

pub enum ShutdownCause {
    Done,
}

impl ShutdownCause {
    pub fn into_close_msg(self) -> OwnedMessage {
        OwnedMessage::Close(match self {
            ShutdownCause::Done => None,
        })
    }
}
