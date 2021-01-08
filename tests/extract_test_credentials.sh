#!/bin/bash -e
# Github Actions has a secret variable for this repository called "SERVICE_ACCOUNT_JSON"
# that contains the base64 encoded credentials json. Create such a secret with "cat file.json | base64 -w 0".
if [ "$(uname)" == "Darwin" ]; then
    echo "$SERVICE_ACCOUNT_JSON" | base64 -D > firebase-service-account.json
else
    echo "$SERVICE_ACCOUNT_JSON" | base64 -d > firebase-service-account.json
fi
# Print the first few lines
head -n 3 firebase-service-account.json
# Sanity check
[[ $(jq -r ".auth_uri" firebase-service-account.json) == "https://accounts.google.com/o/oauth2/auth" ]] || { echo >&2 'Failed to extract firebase-service-account.json'; exit 1; }
jq -e '.api_key' firebase-service-account.json > /dev/null || { echo >&2 'Provided firebase-service-account.json does not have api_key set'; exit 1; }

# Test if the service account user is  still existing
URL=https://www.googleapis.com/service_accounts/v1/jwk/$(jq -r ".client_email" firebase-service-account.json)
echo "Test $URL"
curl -s $URL | jq -e '.keys' > /dev/null || { echo >&2 'Test service account does not exist anymore'; exit 1; }