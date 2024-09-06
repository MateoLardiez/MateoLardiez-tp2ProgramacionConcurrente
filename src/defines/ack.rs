use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq)]
pub struct Ack {
    addr: SocketAddr,
    msg: String,
    type_msg: String,
    num_tries: usize,
}

impl Ack {
    pub fn new(addr: SocketAddr, msg: String, type_msg: String) -> Self {
        Ack {
            addr,
            msg,
            type_msg,
            num_tries: 0,
        }
    }

    pub fn increment_tries(&mut self) {
        self.num_tries += 1;
    }

    pub fn get_num_tries(&self) -> usize {
        self.num_tries
    }

    pub fn get_msg(&self) -> String {
        self.msg.clone()
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn get_type_msg(&self) -> String {
        self.type_msg.clone()
    }

    pub fn is_equal(&self, other_addr: SocketAddr, other_type_msg: String) -> bool {
        self.addr == other_addr && self.type_msg == other_type_msg
    }
}
