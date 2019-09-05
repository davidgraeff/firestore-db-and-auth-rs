#!/bin/bash -e
[[ $(grep -n -r -F "println" src/ | grep -v -c  "///") -eq 0 ]] || { echo >&2 'Please remove all debug println'; grep -n -r -F "println" src/ | grep -v  "///"; exit 1; }
