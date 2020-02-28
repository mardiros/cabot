Feature: As a user, I can speficy a timelimit to fetch a resource

@http
Scenario: I speficy a timeout, and the query take less time
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?900 --max-time 1"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr is empty

@http
Scenario: I speficy a timeout, and the query take less time
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?1100 --max-time 1"
Then the status code is "1"
And stdout is empty
And stderr display
    """
    IO Error: Read Timeout
    """

@http
Scenario: I speficy a timeout, and the query take less time
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?1100 --max-time 2 --read-timeout 1"
Then the status code is "1"
And stdout is empty
And stderr display
    """
    IO Error: Read Timeout
    """

@http
Scenario: I speficy a timeout, and the query take less time
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?1100  --max-time 1 --read-timeout 1"
Then the status code is "1"
And stdout is empty
And stderr display
    """
    IO Error: Read Timeout
    """

@http
Scenario: I speficy a timeout, and the query take less time
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?1100 --read-timeout 1"
Then the status code is "1"
And stdout is empty
And stderr display
    """
    IO Error: Read Timeout
    """


@http
Scenario: I speficy a timeout, and a read timeout what will be overriden by the request timeout
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?1100 --max-time 1 --read-timeout 2"
Then the status code is "1"
And stdout is empty
And stderr display
    """
    IO Error: Read Timeout
    """

@http
Scenario: I speficy a timeout and verbose, and a read timeout what will be overriden by the request timeout
Given cabot
When I run "cabot http://127.0.0.1:8000/timeout?1100 -v --max-time 1"
Then the status code is "1"
And stdout is empty
And stderr display
    """
    * Read timeout is greater than request timeout, overridden (1000ms)
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /timeout?1100 HTTP/1.1
    > User-Agent: cabot/0.6.0
    > Connection: close
    >
    IO Error: Read Timeout
    """

