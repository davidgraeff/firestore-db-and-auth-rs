# Integration Tests

To perform a full integration test, you need a valid "firebase-service-account.json" file.

The tests will create a Firebase user with the ID "Io2cPph06rUWM3ABcIHguR3CIw6v1" and write and read a document to/from "tests/test".
Ensure read/write access via firebase rules.

A refresh and access token is generated.
The refresh token is stored in "refresh-token-for-tests.txt" and will be reused for further tests.
The reason being that Google allows only about [50 simultaneous refresh tokens at any time](https://developers.google.com/identity/protocols/OAuth2#expiration), so we do not want to create a new one each test run.
