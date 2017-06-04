//! Default values
//!
//! [see whats inside](../../src/cabot/constants.rs.html).

/// Version of cabot
pub const VERSION: &'static str = "0.1.2";

/// Default user agent `cabot/{cabot-version}`
pub const USER_AGENT: &'static str = "cabot/0.1.2";


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_user_agent() {
        let ua = format!("cabot/{}", VERSION);
        assert_eq!(USER_AGENT, ua.as_str());
    }
}
