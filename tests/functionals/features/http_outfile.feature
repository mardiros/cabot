Feature: As a user, I can store the http content to a specified file.

@http
Scenario: Read http that have neither content-length, not chunked
Given cabot
When I run "cabot http://127.0.0.1:8000/lorem-ipsum -o outfile.tmp"
Then the status code is "0"
And stdout is empty
And stderr is empty
And tmpfile contains
    """
    Lorem ipsum dolor sit amet, consectetur adipiscing elit.
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
