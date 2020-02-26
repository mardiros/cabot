//! Default values
//!
//! [see whats inside](../../src/cabot/constants.rs.html).

/// Version of cabot
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NUMBER_OF_REDIRECT: u8 = 16;

pub const DNS_LOOKUP_TIMEOUT: u64 = 5;
pub const CONNECT_TIMEOUT: u64 = 15;
pub const READ_TIMEOUT: u64 = 10;
pub const REQUEST_TIMEOUT: u64 = 0;

pub fn user_agent() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Size of the buffer while reading http socket
#[cfg(feature = "functional_tests")]
pub const BUFFER_PAGE_SIZE: usize = 4;

// Size of the buffer while reading http socket
#[cfg(not(feature = "functional_tests"))]
pub const BUFFER_PAGE_SIZE: usize = 2048;
