
Feature: As a user, I can read the help message

Scenario: read command line output
Given cabot
When I run "cabot --help"
Then I can read the command options
