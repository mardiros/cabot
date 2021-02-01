# cabot

[![Build Status](https://travis-ci.org/mardiros/cabot.svg?branch=master)](https://travis-ci.org/mardiros/cabot)
[![Current Crates.io Version](https://img.shields.io/crates/v/cabot.svg)](https://crates.io/crates/cabot)
[![dependency status](https://deps.rs/repo/github/mardiros/cabot/status.svg)](https://deps.rs/repo/github/mardiros/cabot)
[![Latests Documentation](https://docs.rs/cabot/badge.svg)](https://docs.rs/crate/cabot)

 cabot is a learning rust project made on my free time,without the pretention 
 to be production used.
 
## Cabot is a simple Rust library to perform HTTP call.

Take a look at [the documentation for usage](https://docs.rs/cabot/).

Unlike known alternatives, cabot does not rely on OpenSSL to perform https,
but use [rustls](https://crates.io/crates/rustls) instead.

## Cabot is also a command line tool ala curl.

Usge:

   cabot URL

Use `-h` for more options.

## License

BSD 3-Clause License

## Known Alternatives

There are many projects that should be used instead of mine.

 * [surf](https://crates.io/crates/surf) - use async std too
 * [reqwest](https://crates.io/crates/reqwest) - the most used one
 * [ureq](https://github.com/algesten/ureq) - a minimal http client
 * [curl](https://crates.io/crates/curl) - bindings of libcurl

See also [a smoke tested lists of http client in rust](https://medium.com/@shnatsel/smoke-testing-rust-http-clients-b8f2ee5db4e6)
for more choice.

## What this name ?

A cabot is not a certificate authority bot, it is a french word for
a dog, a mutt actually. You throw the ball, he do the rest.
