# Firestore API and Auth

<img align="right" src="./doc/logo.png" />

[![Build Status](https://travis-ci.org/servo/bincode.svg)](https://travis-ci.org/davidgraeff/firestore-db-and-auth-rs)
[![](https://meritbadge.herokuapp.com/firestore-db-and-auth)](https://crates.io/crates/firestore-db-and-auth)
[![](https://img.shields.io/badge/license-MIT-blue.svg)](http://opensource.org/licenses/MIT)

This crate allows easy access to your Google Firestore DB via service account or OAuth impersonated Google Firebase Auth credentials.

Features:
* A CRUD subset of the Firestore v1 API
* Optionally handles authentication and token refreshing for you
* Support for the downloadable Google service account json file from [Google Clound console](https://console.cloud.google.com/apis/credentials/serviceaccountkey).
  (See https://cloud.google.com/storage/docs/reference/libraries#client-libraries-install-cpp)

Usecases:
* Strictly typed document read/write/query access
* Cloud functions (Google Compute, AWS Lambda) access to Firestore

Limitations:
* Listening to document / collection changes is not yet possible

### Document operations

This crate operates on DTOs (Data transfer objects) for type-safe operations on your Firestore DB.

```rust
use firestore_db_and_auth::{credentials, sessions};

/// A document structure for demonstration purposes
#[derive(Debug, Serialize, Deserialize)]
struct DemoDTO {
    a_string: String,
    an_int: u32,
}

let obj = DemoDTO {
    a_string: "abcd".to_owned(),
    an_int: 14,
};

/// Write the given object with the document id "service_test" to the "tests" collection.
/// You do not need to provide a document id (use "None" instead) and let Firestore generate one for you.
/// 
/// In either way a document is created or updated (overwritten).
/// 
/// The method will return document metadata (including a possible generated document id)
let result = documents::write(&mut session, "tests", Some("service_test"), &obj)?;

println!("id: {}, created: {}, updated: {}", result.document_id, result.create_time, result.updated_time);
```

If you want to read the document with the id "service_test" from the Firestore "tests" collection you would do this:

```rust
let obj : DemoDTO = documents::read(&mut session, "tests", "service_test")?;
```

For listing all documents of the "tests" collection you want to use the `List` struct which implements the `Iterator` trait.
It will hide the complexity of the paging API and fetches new documents when necessary:

```rust
let values: documents::List<DemoDTO, _> = documents::list(&mut session, "tests");
for doc_result in values {
    // The document is wrapped in a Result<> because fetching new data could have caused errors
    let doc = doc_result?;
    println!("{:?}", doc);
}
```

*Note:* The resulting list or list cursor is a snapshot view with a limited lifetime.
You cannot keep the iterator for long or expect new documents to appear in an ongoing iteration.

For quering the database you would use the `query` method.
In the following example the collection "tests" is queried for document(s) with the "id" field equal to "Sam Weiss".

```rust
let objs : Vec<DemoDTO> = documents::query(&mut session, "tests", "Sam Weiss", dto::FieldOperator::EQUAL, "id")?;
```

*Note:* The query method returns a vector, because a query potentially returns multiple matching documents.


### Document access via service account

1. Download the service accounts credentials file and store it as "firebase-service-account.json".
   The file should contain `"private_key_id": ...`.
1. Add another field `"api_key" : "YOUR_API_KEY"` and replace YOUR_API_KEY with your *Web API key*, to be found in the [Google Firebase console](https://console.firebase.google.com) in "Project Overview -> Settings - > General".

```rust
use firestore_db_and_auth::{credentials, sessions};

/// Create credentials object. You may as well do that programmatically.
let cred = credentials::Credentials::from_file("firebase-service-account.json")
    .expect("Read credentials file");

/// To use any of the Firestore methods, you need a session first. You either want
/// an impersonated session bound to a Firebase Auth user or a service account session.
let mut session = sessions::service_account::Session::new(&cred)
    .expect("Create a service account session");
```

**Mutable session variable?**: Access (bearer) tokens have a limited lifetime, usually about an hour.
They need to be refreshed via a refresh token, which is also part of the session object.
When you perform a call to an API, the session will automatically refresh your access token if necessary,
and therefore requires the session object to be mutable.

### Document access via a firebase user access / refresh token or via user_id

You can create a user session in various ways.
If you just have the firebase Auth user_id, you would follow these steps:

```rust
use firestore_db_and_auth::{credentials, sessions};

/// Create credentials object. You may as well do that programmatically.
let cred = credentials::Credentials::from_file("firebase-service-account.json")
    .expect("Read credentials file");

/// To use any of the Firestore methods, you need a session first.
/// Create an impersonated session bound to a Firebase Auth user via your service account credentials.
let mut session = sessions::user::Session::by_user_id(&cred, "the_user_id")
    .expect("Create a user session");
```

If you have a valid refresh token already and want to generate an access token (and a session object), you do this instead:

```rust
let refresh_token = "fkjandsfbajsbfd;asbfdaosa.asduabsifdabsda,fd,a,sdbasfadfasfas.dasdasbfadusbflansf";
let mut session = sessions::user::Session::by_refresh_token(&cred, &refresh_token)?;
```

The last way to retrieve a session object is by providing a valid access token like so:

```rust
let access_token = "fkjandsfbajsbfd;asbfdaosa.asduabsifdabsda,fd,a,sdbasfadfasfas.dasdasbfadusbflansf";
let mut session = sessions::user::Session::by_access_token(&cred, &access_token)?;
```

The `by_access_token` method will fail if the token is not valid anymore.
Please note that a session created this way is not able to automatically refresh its access token, because there
is no refresh_token associated with it.

### About the integrated authentication implementation

The `sessions` module of this create allows you to create access tokens for using the Firestore API.

If you use service account credentials and a "service_account" session, internally a JWS (Json Web signature)
is created. A jws is a fully signed JWT (Java web Token) via the private key of the service account.

Google APIs accept such a JWS as bearer token.

If you use user sessions via `Session::by_user_id` a custom JWT is generated,
again signed via the private key of the service account and exchanged via the Firestore Auth API into a
refresh token and access token tuple (like in the OAuth2 Code Grant flow).

If you use user sessions via `Session::by_refresh_token` and `Session::by_access_token` the provided token is validated via the public
keys of the corresponding Google service account (https://www.googleapis.com/service_accounts/v1/jwk/service.account@address).
The public keys are of course cached the very first time you create a `credentials::Credentials` object.

**Security related note**: Depending on your SSL setup and if you have host name / certificate verification for `https://www.googleapis.com` enabled, Man-in-the-middle attacks are impossible.

### Use your own authentication implementation

You do not need the `sessions` module for using the Firestore API of this crate.
All Firestore methods in `documents` expect an object that implements the `FirebaseAuthBearer` trait.

That trait looks like this:

```rust
pub trait FirebaseAuthBearer<'a> {
    fn projectid(&'a self) -> &'a str;
    fn bearer(&'a mut self) -> &'a str;
}
```

Just implement this trait for your own data structure and provide the Firestore project id and a valid access token.

### Http Rocket Server integration

Because the `sessions` module of this crate is already able to verify access tokens,
it was not much more work to turn this into a Rocket 0.4+ Guard.

The implemented Guard (enabled by the feature "rocket_support") allows access to http paths
if the provided http "Authorization" header contains a valid "Bearer" token.
The above mentioned validations on the token are performed.

Example usage:

```rust
use firestore_db_and_auth::{credentials, sessions::service_account, rocket::guard::ApiKey};

fn main() {
    let credentials = credentials::Credentials::from_file("firebase-service-account.json").unwrap();
    rocket::ignite().manage(credentials).mount("/", routes![hello, hello_not_logged_in]).launch();
}

/// And an example route could be:
#[get("/hello")]
fn hello<'r>(_api_key: ApiKey,) -> &'r str {
    // ApiKey is a single value tuple with a sessions::user::Session object inside
    "you are logged in"
}

#[get("/hello")]
fn hello_not_logged_in<'r>() -> &'r str {
    "you are not logged in"
}
```

## Usage in cloud functions

The start up time is crucial for cloud functions.

The usual start up process includes 
* downloading the public jwks keys from a Google server,
* read in the json credentials file,
* create a service account session by creating a custom jwt token.

Even on optimal conditions you will experience a few hundred milliseconds delay before Firestore can be accessed.

1. This crate ships with a helper tool that creates a binary serialized  `service_acount::Session` (stored in `service_account_session.bin`).
   Head to your directory that contains your `firebase-service-account.json` and execute `cargo run --bin binary_session`.

2. At runtime you avoid any file reads, network calls, jwt creation by using the following macro:
   ```rust
   use firestore_db_and_auth::{sessions, from_binary};

   let session : sessions::service_account::Session = from_binary!("../../service_account_session.bin");
   ```
   Note: The file is located relative to the current source file. 


The above macro uses `std::include_bytes` internally to embed the predefined session into your executable.

If you need to call a Firestore API on behalf of a Firebase Auth user via `user::Session`,
you can at any time access  the `credentials::Credentials` object from that `service_account::Session`.

## Testing

To perform a full integration test, you need a valid "firebase-service-account.json" file.
The tests will create a Firebase user with the ID "Io2cPph06rUWM3ABcIHguR3CIw6v1" and write and read a document to/from "tests/test".

If you are using firebase rules (recommended!), please ensure that the mentioned user id has access to the "tests" collection.

A refresh and access token is generated.
The refresh token is stored in "refresh-token-for-tests.txt" and will be reused for further tests.
The reason being that Google allows only about [50 simultaneous refresh tokens at any time](https://developers.google.com/identity/protocols/OAuth2#expiration), so we do not want to create a new one each test run.

If the tests run through with your "firebase-service-account.json" file, you are correctly setup and ready to use this library.

Start test runs with `cargo test` as usual.

## Further development

Maintenance status: Stable

What can be done to make this crate more awesome:

* The communication efficieny can be improved by using gRPC/Protobuf instead of HTTP/Json
* More DTOs (Data transfer objects) and convenience methods should be exposed for the Firebase Auth API
* Nice to have: Transactions, batch_get and streaming (listen) support for Firestore

This library does not have the ambition to mirror the http/gRPC API 1:1.
There are auto-generated libraries for this purpose.
