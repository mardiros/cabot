//! DNS Resolution

use std::time::Duration;

use async_std::io::{self, stderr};
use async_std::net::{SocketAddr, ToSocketAddrs};
use async_std::prelude::*;
use log::Level::Info;

use super::results::{CabotError, CabotResult};

pub struct Resolver {
    verbose: bool,
}

impl Resolver {
    pub fn new(verbose: bool) -> Self {
        Resolver { verbose }
    }
    pub async fn get_addr(
        &self,
        authority: &str,
        ipv4: bool,
        ipv6: bool,
        dns_timeout: u64,
    ) -> CabotResult<SocketAddr> {
        debug!("Resolving TCP Endpoint for authority {}", authority);

        let addrs = io::timeout(Duration::from_millis(dns_timeout), async {
            authority.to_socket_addrs().await
        })
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::TimedOut => CabotError::DNSLookupError("Timeout".to_owned()),
            io::ErrorKind::Other => CabotError::DNSLookupError("Host does not exists".to_owned()),
            _ => CabotError::DNSLookupError(format!("{}", err)),
        })?;

        let addr = addrs
            .filter(|addr| (ipv4 && addr.is_ipv4()) || (ipv6 && addr.is_ipv6()))
            .next()
            .ok_or(CabotError::DNSLookupError(
                "No IP found for this host".to_owned(),
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
            .await
            .unwrap();
        }
        Ok(addr)
    }
}
