//! DNS Resolution

use log::LogLevel::Info;
use std::io::{stderr, Write};
use std::net::{SocketAddr, ToSocketAddrs};

use super::results::{CabotError, CabotResult};

pub struct Resolver {
    verbose: bool,
}

impl Resolver {
    pub fn new(verbose: bool) -> Self {
        Resolver { verbose }
    }
    pub fn get_addr(&self, authority: &str) -> CabotResult<SocketAddr> {
        debug!("Resolving TCP Endpoint for authority {}", authority);
        let mut addrs = authority
            .to_socket_addrs()
            .map_err(|err| CabotError::IOError(err))?;
        let addr = addrs.next(); // get first item from iterator
        if addr.is_none() {
            return Err(CabotError::DNSLookupError(
                "Host does not exists".to_owned(),
            ));
        }
        let addr = addr.unwrap();
        if log_enabled!(Info) {
            info!("Authority {} has been resolved to {}", authority, addr);
        } else if self.verbose {
            writeln!(
                &mut stderr(),
                "* Authority {} has been resolved to {}",
                authority,
                addr
            ).unwrap();
        }
        Ok(addr)
    }
}
