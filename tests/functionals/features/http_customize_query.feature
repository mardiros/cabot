Feature: As a user, I can customize verb, header and send a request body

@http
Scenario: Send a specific header
Given cabot
When I run "cabot http://127.0.1:8000/echo -H 'Header-Name: whynot'"
Then the status code is "0"
And stdout display
    """
    CONTENT_TYPE: text/plain
    HTTP_CONNECTION: close
    HTTP_HEADER_NAME: whynot
    HTTP_HOST: localhost
    HTTP_USER_AGENT: cabot/0.5.0
    PATH_INFO: /echo
    REQUEST_METHOD: GET
    """
And stderr is empty

@http
Scenario: Send multiple specific header
Given cabot
When I run "cabot http://127.0.1:8000/echo -H 'Header-Name: A' -H 'Header-Name: B' -H 'Header-Other: C'"
Then the status code is "0"
And stdout display
    """
    CONTENT_TYPE: text/plain
    HTTP_CONNECTION: close
    HTTP_HEADER_NAME: A,B
    HTTP_HEADER_OTHER: C
    HTTP_HOST: localhost
    HTTP_USER_AGENT: cabot/0.5.0
    PATH_INFO: /echo
    REQUEST_METHOD: GET
    """
And stderr is empty

@http
Scenario: Send header and body
Given cabot
When I run "cabot http://127.0.1:8000/echo -X POST -H 'Content-Type: application/json' -d '{"a": "b"}'"
Then the status code is "0"
And stdout display
    """
    CONTENT_LENGTH: 10
    CONTENT_TYPE: application/json
    HTTP_CONNECTION: close
    HTTP_HOST: localhost
    HTTP_USER_AGENT: cabot/0.5.0
    PATH_INFO: /echo
    REQUEST_METHOD: POST
    body: {"a": "b"}
    """
And stderr is empty

@http
Scenario: Send custom user agent
Given cabot
When I run "cabot http://127.0.1:8000/echo -A Mozilla/5.0"
Then the status code is "0"
And stdout display
    """
    CONTENT_TYPE: text/plain
    HTTP_CONNECTION: close
    HTTP_HOST: localhost
    HTTP_USER_AGENT: Mozilla/5.0
    PATH_INFO: /echo
    REQUEST_METHOD: GET
    """
And stderr is empty
