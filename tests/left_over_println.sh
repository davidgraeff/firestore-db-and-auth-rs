#!/bin/bash -e
[[ $(fgrep -r 'println' src/ | wc -l) -eq 0 ]] || { echo >&2 'Please remove all debug println!'; exit 1; }