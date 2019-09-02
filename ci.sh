#!/bin/bash
[[ $(fgrep -r 'println' src/ | wc -l) -eq 0 ]] || { echo >&2 'Left over println!'; exit 1; }
echo $SERVICE_ACCOUNT_JSON | base64 -d > firebase-service-account.json
[[ $(jq -r ".auth_uri" firebase-service-account.json) == "https://accounts.google.com/o/oauth2/auth" ]] || { echo >&2 'Failed to extract firebase-service-account.json'; exit 1; }