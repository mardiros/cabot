//! DNS Resolution

use std::io::{stderr, Write};
use std::time::Duration;

use async_std::io;
use async_std::net::{SocketAddr, ToSocketAddrs};
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
    ) -> CabotResult<SocketAddr> {
        debug!("Resolving TCP Endpoint for authority {}", authority);

        let addrs = io::timeout(Duration::from_secs(10), async {
            authority.to_socket_addrs().await
        })
        .await?;

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
