pub type WebSocketServer = websocket::sync::Server<websocket::server::NoTlsAcceptor>;
pub type WebSocketUpgrade = websocket::server::upgrade::WsUpgrade<
    std::net::TcpStream,
    Option<websocket::server::upgrade::sync::Buffer>,
>;
pub type WebSocketClient = websocket::sync::Client<std::net::TcpStream>;
pub type WebSocketWriter = websocket::sync::sender::Writer<std::net::TcpStream>;
pub type WebSocketReader = websocket::sync::receiver::Reader<std::net::TcpStream>;
