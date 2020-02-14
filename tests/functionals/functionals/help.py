from behave import *

@then('I can read the command options')
def read_options(context):
    assert context.stash['result'].returncode == 0
    stdout = context.stash['result'].stdout
    assert 'cabot [FLAGS] [OPTIONS] <URL>' in stdout

