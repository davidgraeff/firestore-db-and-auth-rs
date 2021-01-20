# Integration Tests

To perform a full integration test, you need a valid "firebase-service-account.json" file.

1. Create and download one of the type "firebase-adminsdk" at [Google Clound console](https://console.cloud.google.com/apis/credentials/serviceaccountkey) and store it as "firebase-service-account.json".
   The file should contain `"private_key_id": ...`. Store the file in the repository root.
2. Add another field `"api_key" : "YOUR_API_KEY"` and replace YOUR_API_KEY with your *Web API key*, to be found in the [Google Firebase console](https://console.firebase.google.com) in "Project Overview -> Settings - > General".
3. The tests will create a Firebase user with the ID "Io2cPph06rUWM3ABcIHguR3CIw6v1" and write and read a document to/from "tests/test".
   Ensure read/write access via firebase rules ("Cloud Firestore -> Rules - > Edit rules"). For example use the rule snippet below:

```
service cloud.firestore {
  match /databases/{database}/documents {
  	// Integration test user: Allow access to tests collection
    match /tests/{documents=**} {
      allow read, write: if request.auth != null && request.auth.uid == "Io2cPph06rUWM3ABcIHguR3CIw6v1"
    }
  }
}
```



A refresh and access token is generated.
The refresh token is stored in "refresh-token-for-tests.txt" and will be reused for further tests.
The reason being that Google allows only about [50 simultaneous refresh tokens at any time](https://developers.google.com/identity/protocols/OAuth2#expiration), so we do not want to create a new one for each test run.

The original repository of this crate uses a "firebase-service-account.json"
that is stored in base64 as Github CI secret.
Have a look at "tests/extract_test_credentials.sh" to see how the secret environment variable is
base64 decoded and stored as file again.