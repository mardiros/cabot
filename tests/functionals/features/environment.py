import os
import pathlib
from shutil import copy2
import subprocess

from behave import *

from functionals.fixtures import wsgi

def run_command(context):
    def run_command_impl(command):
        return subprocess.run(
            command.split(),
            capture_output=True,
            text=True,
        )
    return run_command_impl


def before_all(context):

    test_dir = pathlib.Path(__file__).resolve().parent.parent
    test_dir.joinpath('cabot').unlink(missing_ok=True)
    working_dir = test_dir.parent.parent
    os.chdir(working_dir)
    subprocess.run(['cargo', 'build', '--features', 'functional_tests'])
    copy2(
        working_dir.joinpath('target', 'release', 'cabot'),
        test_dir,
    )
    os.chdir(test_dir)
    os.environ['PATH'] += os.pathsep + str(test_dir)

    wsgi.setUp()


def before_scenario(context, scenario):
    context.stash = {}
    context.run = run_command(context)


def after_all(context):
    wsgi.tearDown()
