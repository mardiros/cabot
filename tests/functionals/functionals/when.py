"""Generic when statements."""
from behave import when

@when('I run "{command}"')
def run_command(context, command):
    """Run the command."""
    context.stash['result'] = context.run(command)
