#!/bin/bash
while true; do
  nc -l 127.0.0.1 12345 < basic_data;
done