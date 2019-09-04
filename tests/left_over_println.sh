#!/bin/bash -e
[[ $(fgrep -n -r "println" src/ | grep -v  "///" | wc -l) -eq 0 ]] || { echo >&2 'Please remove all debug println'; fgrep -n -r "println" src/ | grep -v  "///"; exit 1; }
