# Cabot Functional Tests Suite.

This tests suite tests the cabot command line tools, it use the 
[behave](https://behave.readthedocs.io/en/latest/) tests.

## Requirements

 * Python 3.8
 * [Poetry](https://python-poetry.org/)

The tests has been written with python 3.8 but it should be easy to downgrade
it as it does not requires new shining features.

## Installation

    poetry install

## Run

    poetry run behave
