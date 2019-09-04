#!/bin/bash -e
# Github Actions has a secret variable for this repository called "SERVICE_ACCOUNT_JSON"
# that contains the base64 encoded credentials json
if [ "$(uname)" == "Darwin" ]; then
    echo $SERVICE_ACCOUNT_JSON | base64 -D > firebase-service-account.json
else
    echo $SERVICE_ACCOUNT_JSON | base64 -d > firebase-service-account.json
fi
# Print the first few characters
head -n 3 firebase-service-account.json
# Sanity check
[[ $(jq -r ".auth_uri" firebase-service-account.json) == "https://accounts.google.com/o/oauth2/auth" ]] || { echo >&2 'Failed to extract firebase-service-account.json'; exit 1; }