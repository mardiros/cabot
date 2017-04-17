use std::net::{ToSocketAddrs, SocketAddr};

pub struct Resolver {}

impl Resolver {
    pub fn new() -> Self {
        Resolver {}
    }
    pub fn get_addr(&self, host: &str) -> SocketAddr {
        host.to_socket_addrs()
            .unwrap() // unwrap result
            .next().unwrap() // get first item from iterator
    }
}
