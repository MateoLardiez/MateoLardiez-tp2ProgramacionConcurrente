use actix::prelude::Message;
use tokio::net::TcpStream;

#[derive(Message)]
#[rtype(result = "()")]
pub struct HandleConnection {
    pub stream: TcpStream,
    pub addr: std::net::SocketAddr,
}

impl HandleConnection {
    pub fn new(stream: TcpStream, addr: std::net::SocketAddr) -> HandleConnection {
        HandleConnection { stream, addr }
    }
}
