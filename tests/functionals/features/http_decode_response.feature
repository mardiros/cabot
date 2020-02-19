
Feature: As a user, I can get http response body on stdout

@http
Scenario: Read http that have neither content-length, not chunked
Given cabot
When I run "cabot http://127.0.0.1:8000/no-length"
Then the status code is "0"
And stdout display
    """
    Content without header for its length.
    """
And stderr is empty

@http @verbose
Scenario: Read http that have neither content-length, not chunked
Given cabot
When I run "cabot -v http://127.0.0.1:8000/no-length"
Then the status code is "0"
And stdout display
    """
    Content without header for its length.
    """
And stderr display
    """
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /no-length HTTP/1.1
    > User-Agent: cabot/0.4.0
    > Connection: close
    >
    < HTTP/1.1 200 OK
    < Server: Dummy-Server
    < Date: Mon, 17 Feb 2020 21:11:21 GMT
    < Content-type: text/plain; charset=utf-8
    """

@http
Scenario: Read http that have content-length
Given cabot
When I run "cabot http://127.0.0.1:8000/with-length"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr is empty

@http @verbose
Scenario: Read http that have content-length
Given cabot
When I run "cabot -v http://127.0.0.1:8000/with-length"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr display
    """
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /with-length HTTP/1.1
    > User-Agent: cabot/0.4.0
    > Connection: close
    >
    < HTTP/1.1 200 OK
    < Server: Dummy-Server
    < Date: Mon, 17 Feb 2020 21:11:21 GMT
    < Content-type: text/plain; charset=utf-8
    < Content-Length: 14
    """


@http @verbose
Scenario: Read http chunked response with chunked smaller than the buffer.
Given cabot
When I run "cabot -v http://127.0.0.1:8000/small-chunked"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr display
    """
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /small-chunked HTTP/1.1
    > User-Agent: cabot/0.4.0
    > Connection: close
    >
    < HTTP/1.1 200 OK
    < Server: Dummy-Server
    < Date: Mon, 17 Feb 2020 21:11:21 GMT
    < Content-type: text/plain; charset=utf-8
    < Transfer-Encoding: chunked
    """

@http
Scenario: Read http chunked response with chunked smaller than the buffer.
Given cabot
When I run "cabot http://127.0.0.1:8000/large-chunked"
Then the status code is "0"
And stdout display
    """
    It is working.
    With chunked larger than the buffer.
    """
And stderr is empty
