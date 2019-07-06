//! DNS Resolution

use log::Level::Info;
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
    pub fn get_addr(&self, authority: &str, ipv4: bool, ipv6: bool) -> CabotResult<SocketAddr> {
        debug!("Resolving TCP Endpoint for authority {}", authority);
        let addrs = authority.to_socket_addrs()?;
        let addr = addrs
            .filter(|addr| (ipv4 && addr.is_ipv4()) || (ipv6 && addr.is_ipv6()))
            .next()
            .ok_or(CabotError::DNSLookupError(
                "Host does not exists".to_owned(),
            ))?;
        if log_enabled!(Info) {
            info!("Authority {} has been resolved to {}", authority, addr);
        } else if self.verbose {
            writeln!(
                &mut stderr(),
                "* Authority {} has been resolved to {}",
                authority,
                addr
            )
            .unwrap();
        }
        Ok(addr)
    }
}
