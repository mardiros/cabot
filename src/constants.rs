//! Default values
//!
//! [see whats inside](../../src/cabot/constants.rs.html).

use regex::Regex;
use regex::bytes::Regex as BytesRegex;

/// Version of cabot
pub const VERSION: &'static str = "0.2.0";

/// Default user agent `cabot/{cabot-version}`
pub const USER_AGENT: &'static str = "cabot/0.2.0";

lazy_static! {
    pub static ref SPLIT_HEADERS_RE: BytesRegex = BytesRegex::new("\r?\n\r?\n").unwrap();
    pub static ref SPLIT_HEADER_RE: Regex = Regex::new("\r?\n").unwrap();
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_user_agent() {
        let ua = format!("cabot/{}", VERSION);
        assert_eq!(USER_AGENT, ua.as_str());
    }
}
