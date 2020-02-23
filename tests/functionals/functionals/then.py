"""Generic then statements."""
from behave import then


@then('the status code is "{status_code}"')
def check_status_code(context, status_code):
    assert context.stash['result'].returncode == int(status_code)


@then('stderr display')
def check_stderr(context):
    for x, y in zip(
        context.stash['result'].stderr.split('\n'), context.text.split('\n')
    ):
        assert x == y


@then('stderr is empty')
def check_empty_stderr(context):
    assert context.stash['result'].stderr == ''


@then('stdout display')
def check_stdout(context):
    for x, y in zip(
        context.stash['result'].stdout.split('\n'), context.text.split('\n')
    ):
        assert x == y


@then('stdout is empty')
def check_empty_stderr(context):
    assert context.stash['result'].stdout == ''


@then('tmpfile contains')
def check_stdout(context):
    with open('outfile.tmp', 'r') as f:
        for x, y in zip(
            f.readlines(), context.text.split('\n')
        ):
            assert x.rstrip('\n') == y
