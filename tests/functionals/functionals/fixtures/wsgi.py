#! /usr/bin/env python3
import subprocess
import time

from wsgiref.util import setup_testing_defaults
from wsgiref.simple_server import (
    make_server,
    WSGIRequestHandler,
    ServerHandler,
)


class MyServerHandler(ServerHandler):
    server_software = 'Dummy-Server'
    http_version = '1.1'

    def set_content_length(self):
        pass

    def start_response(self, status, headers, exc_info=None):
        """'start_response()' callable as specified by PEP 3333"""

        if exc_info:
            try:
                if self.headers_sent:
                    # Re-raise original exception if headers sent
                    raise exc_info[0](exc_info[1]).with_traceback(exc_info[2])
            finally:
                exc_info = None  # avoid dangling circular ref
        elif self.headers is not None:
            raise AssertionError("Headers already set!")

        self.status = status
        self.headers = self.headers_class(headers)
        status = self._convert_string_type(status, "Status")
        assert len(status) >= 4, "Status must be at least 4 characters"
        assert status[:3].isdigit(), "Status message must begin w/3-digit code"
        assert status[3] == " ", "Status message must have a space after code"
        return self.write


class MyWSGIRequestHandler(WSGIRequestHandler):
    def handle(self):
        """Handle a single HTTP request"""

        self.raw_requestline = self.rfile.readline(65537)
        if len(self.raw_requestline) > 65536:
            self.requestline = ''
            self.request_version = ''
            self.command = ''
            self.send_error(414)
            return

        if not self.parse_request():  # An error code has been sent, just exit
            return

        handler = MyServerHandler(
            self.rfile,
            self.wfile,
            self.get_stderr(),
            self.get_environ(),
            multithread=False,
        )
        handler.request_handler = self  # backpointer for logging
        handler.run(self.server.get_app())


class WsgiApp:
    def __init__(self, environ):
        self.environ = environ

    def no_length(self):
        status = '200 OK'
        headers = [
            ('Date', 'Mon, 17 Feb 2020 21:11:21 GMT'),
            ('Content-type', 'text/plain; charset=utf-8'),
        ]
        body = b"""Content without header for its length."""
        return status, headers, body

    def with_length(self):
        status = '200 OK'
        body = b"""It is working."""
        headers = [
            ('Date', 'Mon, 17 Feb 2020 21:11:21 GMT'),
            ('Content-type', 'text/plain; charset=utf-8'),
            ('Content-Length', str(len(body))),
        ]
        return status, headers, body

    def small_chunked(self):
        status = '200 OK'
        body = '\r\n'.join(
            [
                '2',
                'It',
                '3',
                ' is',
                '1',
                ' ',
                '2',
                'wo',
                '2',
                'rk',
                '3',
                'ing',
                '1',
                '.',
                '0',
                '\r\n',
            ]
        ).encode('utf-8')
        headers = [
            ('Date', 'Mon, 17 Feb 2020 21:11:21 GMT'),
            ('Content-type', 'text/plain; charset=utf-8'),
            ('Transfer-Encoding', 'chunked'),
        ]
        return status, headers, body

    def large_chunked(self):
        status = '200 OK'
        body = '\r\n'.join(
            [
                'F',
                'It is working.\n',
                '24',
                'With chunked larger than the buffer.',
                '0',
                '\r\n',
            ]
        ).encode('utf-8')
        headers = [
            ('Date', 'Mon, 17 Feb 2020 21:11:21 GMT'),
            ('Content-type', 'text/plain; charset=utf-8'),
            ('Transfer-Encoding', 'chunked'),
        ]
        return status, headers, body

    def lorem_ipsum(self):
        status = '200 Ok'
        body = b"""Lorem ipsum dolor sit amet, consectetur adipiscing elit.
Nullam interdum, diam in luctus hendrerit, metus arcu rutrum neque, et
fringilla arcu purus non mi. Donec condimentum auctor maximus.
Vivamus pellentesque ullamcorper risus. Vivamus a nibh ante.
Proin eu urna arcu. Nunc et porta felis, ut viverra nisi.
Vestibulum vestibulum, felis id euismod gravida, nulla quam luctus nunc,
sit amet porttitor tellus purus vitae dui. Integer porta tincidunt massa
eget condimentum.
Donec gravida massa at ex semper, nec mattis purus rhoncus.
Mauris dignissim, diam at dignissim vulputate, tortor dolor accumsan metus,
vitae efficitur dui lacus ut nunc.
Maecenas lectus nibh, accumsan vitae lorem non, congue lacinia justo.
Cras auctor sollicitudin varius. Vivamus malesuada lobortis dolor id
sollicitudin.
In orci justo, sollicitudin non imperdiet blandit, semper vel sem.
Nunc et augue sed nulla ultricies molestie quis gravida mi.
Praesent eget euismod est, quis auctor erat.
"""
        headers = [
            ('Date', 'Mon, 17 Feb 2020 21:11:21 GMT'),
            ('Content-type', 'text/plain; charset=utf-8'),
            ('Content-Length', str(len(body))),
        ]
        return status, headers, body

    def redirect_count_down(self):
        count = int(self.environ['QUERY_STRING'])

        status = '302 Found'
        location = (
            'http://127.0.0.1:8000/redirect-count-down?{}'.format(count - 1)
            if count
            else 'http://127.0.0.1:8000/with-length'
        )
        body = "Go see {}".format(location).encode('utf-8')
        headers = [
            ('Date', 'Mon, 17 Feb 2020 21:11:21 GMT'),
            ('Content-type', 'text/plain; charset=utf-8'),
            ('Content-Length', str(len(body))),
            ('Location', location,),
        ]
        return status, headers, body


def wsgi_app(environ, start_response):
    setup_testing_defaults(environ)

    func = environ['PATH_INFO'][1:].replace('-', '_').replace('/', '_')
    status, headers, body = getattr(WsgiApp(environ), func)()
    start_response(status, headers)
    return [body]


process = None


def setUp():
    global process
    process = subprocess.Popen(
        'python ./functionals/fixtures/wsgi.py', shell=True
    )
    time.sleep(0.7)  # with that the wsti server starts


def tearDown():
    process.terminate()


if __name__ == '__main__':
    with make_server(
        '127.0.0.1', 8000, wsgi_app, handler_class=MyWSGIRequestHandler
    ) as httpd:
        print("Serving on port 8000...")
        httpd.serve_forever()
