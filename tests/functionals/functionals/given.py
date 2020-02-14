"""Generic given statements."""

from behave import given

@given('cabot')
def get_cabot(context):
    context.stash = {}

