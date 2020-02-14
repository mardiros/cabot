import os
import pathlib
import subprocess

from behave import *


def before_all(context):
    working_dir = pathlib.Path(__file__).resolve().parent.parent.parent.parent
    os.chdir(working_dir)
    subprocess.run(['cargo', 'build', '--release'])


