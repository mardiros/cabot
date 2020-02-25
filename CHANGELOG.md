# CHANGELOG

## cabot 0.5.0 2019-02-25

 * Update dependencies.

## cabot 0.5.0 2019-02-24

 * Implement redirection.
 * Implement request, dns, connect and read timeout.
 * Fix Chunked enconding.
 * Fix -o option.
 * Add a functionnal tests suite.

## cabot 0.4.0 2019-12-05

 * Use `async-std` instead of `std`
 * Stream https ouput while deciphering, remove useless buffering

## cabot 0.3.0 2019-07-11

 * Implement `Transfer-Encoding: chunked`.
 * Use the `Content-Length` returned to get the number of bytes to read before closing the connection
 * Stream the ouput from the socket to stdout in the CLI
 * Update header separate to be strict with the spec. \r is no more consider optional.

## cabot 0.2.1 2019-07-03

 * Upgrade dependencies.
 * Use the (`env!` macro)[https://doc.rust-lang.org/1.2.0/std/macro.env!.html] to remove 
   duplicated information (from cargo)[https://rurust.github.io/cargo-docs-ru/environment-variables.html#environment-variables-cargo-sets-for-crates].

## cabot 0.2.0 2018-03-16

 * Don't panic in case of network errors.
 * Add a --resolve command line argument to force resolution to a given address, avoid DNS resolution.
 * Add Client.add_authority method to force resolution to a given address, avoid DNS resolution.
 * Internal function `http::http_query` signature changes for the new host resolution feature.

## cabot 0.1.4 2018-01-30

 * Update dependencies.


## cabot 0.1.3 2017-06-09

 * Convert request to bytes instead of string to send it.
 * Split http response headers using bytes regex.


## cabot 0.1.2 2017-06-04

 * CLI - Fix download of binary files.


## cabot 0.1.1 2017-06-03

 * Use (read)[https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read]
   instead of (read_to_end)[https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end]
 * CLI - Flush the output stream when the response is complete.


## cabot 0.1.0 2017-05-29

 * Initial release that handle http and https query with command line and library.

