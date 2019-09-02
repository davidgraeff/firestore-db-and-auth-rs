#!/bin/bash
[[ $(fgrep -r 'println' src/ | wc -l) -eq 0 ]] || { echo >&2 'Left over println!'; exit 1; }
echo $SERVICE_ACCOUNT_JSON | base64 -d > firebase-service-account.json