# Firestore API and Auth

<img alt="Firestore Logo, Copyright by Google" align="right" src="https://github.com/davidgraeff/firestore-db-and-auth-rs/raw/master/doc/logo.png" />

[![Build Status](https://github.com/davidgraeff/firestore-db-and-auth-rs/workflows/Integration/badge.svg)](https://github.com/davidgraeff/firestore-db-and-auth-rs/actions)
[![Build Status](https://github.com/davidgraeff/firestore-db-and-auth-rs/workflows/With%20Rocket/badge.svg)](https://github.com/davidgraeff/firestore-db-and-auth-rs/actions)
[![](https://meritbadge.herokuapp.com/firestore-db-and-auth)](https://crates.io/crates/firestore-db-and-auth)
[![](https://img.shields.io/badge/license-MIT-blue.svg)](http://opensource.org/licenses/MIT)

This crate allows easy access to your Google Firestore DB via service account or OAuth impersonated Google Firebase Auth credentials.

Features:
* Subset of the Firestore v1 API
* Optionally handles authentication and token refreshing for you
* Support for the downloadable Google service account json file from [Google Clound console](https://console.cloud.google.com/apis/credentials/serviceaccountkey).
  (See https://cloud.google.com/storage/docs/reference/libraries#client-libraries-install-cpp)

Usecases:
* Strictly typed document read/write/query access
* Cloud functions (Google Compute, AWS Lambda) access to Firestore

Limitations:
* Listening to document / collection changes is not yet possible

### Cargo features

* `rocket_support`: Enables the rocket guard.
  Only Firestore Auth authorized requests can pass this guard.
  This feature requires rust nightly, because Rocket itself requires nightly.
* `rustls-tls`: Use rustls instead of native-tls (openssl on Linux).
  If you want to compile this crate for musl, this is what you want.
  Don't forget to disable the default features with ` --no-default-features`.

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

Read the document with the id "service_test" from the Firestore "tests" collection:

```rust
let obj : DemoDTO = documents::read(&mut session, "tests", "service_test")?;
```

For listing all documents of the "tests" collection you want to use the `List` struct which implements the `Iterator` trait.
It will hide the complexity of the paging API and fetches new documents when necessary:

```rust
let values: documents::List<DemoDTO, _> = documents::list(&mut session, "tests");
for doc_result in values {
    // The document is wrapped in a Result<> because fetching new data could have failed
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
2. Add another field `"api_key" : "YOUR_API_KEY"` and replace YOUR_API_KEY with your *Web API key*, to be found in the [Google Firebase console](https://console.firebase.google.com) in "Project Overview -> Settings - > General".

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
Please note that a session created this way is not able to automatically refresh its access token.
There is no *refresh_token* associated with it.

### Firestore Auth: Background information

**JWT**: Firestore Auth makes use of the *OAuth Grant Code Flow* and uses *JWT*s (Json web Tokens)
as access tokens. Such a token is signed by Google and consists of a few encoded fields including
a valid-until field. This allows to verify access tokens locally without any database access.

The Firebase API requires an access token, it accepts two types:

1. A custom created JWT, signed with the private key of a Google service account
2. An access token from Firestore Auth, bound to a user (in this crate called "user session")

If you do not have an user session access token, but you need to perform an action
impersonated, this crate offers `Session::by_user_id`. This will again create a custom, signed JWT,
like with option 1, but exchanges this JWT for a refresh token and access token tuple.
The actual database operation will be performed with those tokens.

About token validation:

Validation happens via the public keys of the corresponding Google service account (https://www.googleapis.com/service_accounts/v1/jwk/service.account@address).
The public keys are downloaded and cached the very first time you create a `credentials::Credentials` object.

To avoid this roundtrip on start it is **strongly** recommended to serialize the credentials object to disk.
Find more information further down.

### Use your own authentication implementation

You do not need the `sessions` module for using the Firestore API of this crate.
All Firestore methods in `documents` expect an object that implements the `FirebaseAuthBearer` trait.

That trait looks like this:

```rust
pub trait FirebaseAuthBearer<'a> {
    fn projectid(&'a self) -> &'a str;
    fn bearer(&'a self) -> &'a str;
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
The usual start up procedure includes three IO operations:

* downloading the two public jwks keys from a Google server,
* and read in the json credentials file.

Avoid those by embedding the credentials and public key files into your application.

First download the 2 public key files:

* https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com -> Store as `securetoken.jwks`
* https://www.googleapis.com/service_accounts/v1/jwk/{your-service-account-email} -> Store as `service-account.jwks`

Create a `Credentials` object like so:

```rust
let mut c : Credentials = serde_json::from_str(include_str!("firebase-service-account.json"))?
c.add_jwks_public_keys(serde_json::from_str(include_str!("securetoken.jwks"))?);
c.add_jwks_public_keys(serde_json::from_str(include_str!("service-account.jwks"))?);
```

## Testing

To perform a full integration test, you need a valid "firebase-service-account.json" file.
The tests will create a Firebase user with the ID "Io2cPph06rUWM3ABcIHguR3CIw6v1" and write and read a document to/from "tests/test".

If you are using firebase rules (recommended!), please ensure that the mentioned user id has access to the "tests" collection.

A refresh and access token is generated.
The refresh token is stored in "refresh-token-for-tests.txt" and will be reused for further tests.
The reason being that Google allows only about [50 simultaneous refresh tokens at any time](https://developers.google.com/identity/protocols/OAuth2#expiration), so we do not want to create a new one each test run.

If the tests run through with your "firebase-service-account.json" file, you are correctly setup and ready to use this library.

Start test runs with `cargo test` as usual.

## Documentation

See https://docs.rs/firestore-db-and-auth
Build locally with `cargo +nightly doc --features external_doc,rocket_support`

## Future development

Maintenance status: Stable

What can be done to make this crate more awesome:

* Data streaming via gRPC/Protobuf
* Expose more DTOs (Data transfer objects) and convenience methods.
* Nice to have: Transactions, batch_get support for Firestore

This library does not have the ambition to mirror the http/gRPC API 1:1.
There are auto-generated libraries for this purpose.
