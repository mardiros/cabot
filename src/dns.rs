use std::io::{self, Write, stderr};
use std::net::{ToSocketAddrs, SocketAddr};
use log::LogLevel::Info;

pub struct Resolver {
    verbose: bool
}

impl Resolver {
    pub fn new(verbose: bool) -> Self {
        Resolver {verbose: verbose}
    }
    pub fn get_addr(&self, authority: &str) -> SocketAddr {
        debug!("Resolving TCP Endpoint for authority {}", authority);
        let addr = authority.to_socket_addrs()
            .unwrap() // unwrap result
            .next().unwrap(); // get first item from iterator
        if log_enabled!(Info) {
            info!("Authority {} has been resolved to {}", authority, addr);
        }
        else if self.verbose {
            writeln!(&mut stderr(),
                     "* Authority {} has been resolved to {}",
                     authority, addr).unwrap();
        }
        addr
    }
}
