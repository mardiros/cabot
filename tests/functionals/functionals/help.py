from behave import *

@when('I run "{command}"')
def run_command(context, command):
    context.stash['command'] = command
    context.stash['result'] = ''


@then('I can read the command options')
def read_options(context):
    pass

