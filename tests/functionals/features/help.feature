
Feature: As a user, I can read the help message

@help
Scenario: read command line output without parameter
Given cabot
When I run "cabot"
Then the status code is "1"
And stderr display
    """
    error: The following required arguments were not provided:
        <URL>

    USAGE:
        cabot <URL> --connect-timeout <CONNECT_TIMEOUT> --dns-timeout <DNS_LOOKUP_TIMEOUT> --max-redirs <NUMBER_OF_REDIRECT> --read-timeout <READ_TIMEOUT> --request <REQUEST> --max-time <REQUEST_TIMEOUT> --user-agent <UA>

    For more information try --help

    """

@help
Scenario: read command line output from --help
Given cabot
When I run "cabot --help"
Then the status code is "0"
And stdout display
    """
    cabot 0.4.0
    Guillaume Gauvrit <guillaume@gauvr.it>
    Simple HTTP Client.

    USAGE:
        cabot [FLAGS] [OPTIONS] <URL>

    FLAGS:
        -4, --ipv4       Resolve host names to IPv4 addresses
        -6, --ipv6       Resolve host names to IPv6 addresses
        -v, --verbose    Make the operation more talkative
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -d, --data <BODY>                          Post Data (Using utf-8 encoding)
            --connect-timeout <CONNECT_TIMEOUT>    timeout for the tcp connection [default: 15]
            --dns-timeout <DNS_LOOKUP_TIMEOUT>     timeout for the dns lookup resolution in seconds [default: 5]
        -o, --output <FILE>                        Write to FILE instead of stdout
        -H, --header <HEADER>...                   Pass custom header to server
            --max-redirs <NUMBER_OF_REDIRECT>      max number of redirection before returning a response [default: 16]
            --read-timeout <READ_TIMEOUT>          timeout for the tcp read in seconds [default: 10]
        -X, --request <REQUEST>                    Specify request command to use [default: GET]
            --max-time <REQUEST_TIMEOUT>           timeout for the whole http request in seconds (0 means no timeout)
                                                   [default: 0]
            --resolve <RESOLVE>...                 <host:port:address> Resolve the host+port to this address
        -A, --user-agent <UA>                      The user-agent HTTP header to use [default: cabot/0.4.0]

    ARGS:
        <URL>    URL to request

    """
