# CHANGELOG

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

