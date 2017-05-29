# cabot

[![Build Status](https://travis-ci.org/mardiros/cabot.svg?branch=master)](https://travis-ci.org/mardiros/cabot)
[![Current Crates.io Version](https://img.shields.io/crates/v/cabot.svg)](https://crates.io/crates/cabot)
[![Latests Documentation](https://docs.rs/cabot/badge.svg)](https://docs.rs/crate/cabot)

## Cabot is a simple RUST library to perform HTTP call.

Take a look at [the documentation for usage](https://docs.rs/cabot/).

Unlike known alternatives, cabot does not rely on OpenSSL to perform https,
but use [rustls](https://crates.io/crates/rustls) instead.

## Cabot is also a command line tool ala curl: `cabot <url>`

Use `-h` for more options

## License

BSD 3-Clause License

## Known Alternatives

 * [reqwest](https://crates.io/crates/reqwest)
 * [curl](https://crates.io/crates/curl)
 * [requests](https://crates.io/crates/requests)
