Feature: As a user, I can get http response body after many redirection

@http
Scenario: Redirect until I get a response
Given cabot
When I run "cabot http://127.0.0.1:8000/redirect-count-down?8"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr is empty

@http
Scenario: Redirect until I get a response with verbose
Given cabot
When I run "cabot -v http://127.0.0.1:8000/redirect-count-down?3"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr display
    """
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?3 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?2 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?1 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?0 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /with-length HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    < HTTP/1.1 200 OK
    < Server: Dummy-Server
    < Date: Mon, 17 Feb 2020 21:11:21 GMT
    < Content-type: text/plain; charset=utf-8
    < Content-Length: 14
    """

@http
Scenario: Redirect until I get a response
Given cabot
When I run "cabot http://127.0.0.1:8000/redirect-count-down?8"
Then the status code is "0"
And stdout display
    """
    It is working.
    """
And stderr is empty

@http
Scenario: Redirect until It attempt the default value
Given cabot
When I run "cabot http://127.0.0.1:8000/redirect-count-down?20"
Then the status code is "1"
And stderr display
    """
    Maximum redirection attempt: 16
    """
And stdout is empty

@http
Scenario: Redirect until It attempt the default value with verbose flag
Given cabot
When I run "cabot http://127.0.0.1:8000/redirect-count-down?50 -v"
Then the status code is "1"
And stderr display
    """
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?50 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?49 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?48 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?47 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?46 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?45 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?44 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?43 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?42 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?41 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?40 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?39 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?38 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?37 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?36 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?35 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?34 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Maximum redirects followed (16)
    Maximum redirection attempt: 16
    """
And stdout is empty


@http
Scenario: Redirect until It attempt the value set
Given cabot
When I run "cabot http://127.0.0.1:8000/redirect-count-down?5 --max-redirs 2"
Then the status code is "1"
And stderr display
    """
    Maximum redirection attempt: 2
    """

@http
Scenario: Redirect until It attempt the value set with verbose flag
Given cabot
When I run "cabot http://127.0.0.1:8000/redirect-count-down?5 -v --max-redirs 2"
Then the status code is "1"
And stderr display
    """
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?5 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?4 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Authority 127.0.0.1:8000 has been resolved to 127.0.0.1:8000
    > GET /redirect-count-down?3 HTTP/1.1
    > User-Agent: cabot/0.5.0
    > Connection: close
    >
    * Maximum redirects followed (2)
    Maximum redirection attempt: 2
    """