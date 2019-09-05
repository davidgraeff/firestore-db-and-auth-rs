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

* `rustls-tls`: Use rustls instead of native-tls (openssl on Linux).
  If you want to compile this crate for musl, this is what you want.
  Don't forget to disable the default features with `--no-default-features`.

* `rocket_support`: [Rocket](https://rocket.rs/) is a web framework.
  This features enables rocket integration and adds a [Request Guard](https://rocket.rs/v0.4/guide/requests/#request-guards).
  Only Firestore Auth authorized requests can pass this guard.
  This feature requires rust nightly, because Rocket itself requires nightly.

### Document operations

This crate operates on DTOs (Data transfer objects) for type-safe operations on your Firestore DB.

```rust
use firestore_db_and_auth::{Credentials, ServiceSession, documents, errors::Result};

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

fn main() -> Result<()> {
    let cred = Credentials::from_file("firebase-service-account.json")?;
    let session = ServiceSession::new(&cred)?;
        
    /// Write the given object with the document id "service_test" to the "tests" collection.
    /// You do not need to provide a document id (use "None" instead) and let Firestore generate one for you.
    /// 
    /// In either way a document is created or updated (overwritten).
    /// 
    /// The method will return document metadata (including a possible generated document id)
    let result = documents::write(&session, "tests", Some("service_test"), &obj)?;

    println!("id: {}, created: {}, updated: {}", result.document_id, result.create_time, result.updated_time);
}
```

Read the document with the id "service_test" from the Firestore "tests" collection:

```rust
let obj : DemoDTO = documents::read(&session, "tests", "service_test")?;
```

For listing all documents of the "tests" collection you want to use the `List` struct which implements the `Iterator` trait.
It will hide the complexity of the paging API and fetches new documents when necessary:

```rust
use firestore_db_and_auth::{documents};

let values: documents::List<DemoDTO, _> = documents::list(&session, "tests");
for doc_result in values {
    // The document is wrapped in a Result<> because fetching new data could have failed
    let (doc, _metadata) = doc_result?;
    println!("{:?}", doc);
}
```

*Note:* The resulting list or list cursor is a snapshot view with a limited lifetime.
You cannot keep the iterator for long or expect new documents to appear in an ongoing iteration.

For quering the database you would use the `query` method.
In the following example the collection "tests" is queried for document(s) with the "id" field equal to "Sam Weiss".

```rust
use firestore_db_and_auth::{documents, dto};

let objs : Vec<DemoDTO> = documents::query(&session, "tests", "Sam Weiss", dto::FieldOperator::EQUAL, "id")?;
```

*Note:* The query method returns a vector, because a query potentially returns multiple matching documents.

### Error handling

The returned `Result` will have a `FirebaseError` set in any error case.
This custom error type wraps all possible errors (IO, Reqwest, JWT errors etc)
and Google REST API errors. If you want to specifically check for an API error,
you could do so:

```rust
use firestore_db_and_auth::{documents, errors::FirebaseError};

let r = documents::delete(&session, "tests/non_existing", true);
if let Err(e) = r.err() {
    if let FirebaseError::APIError(code, message, context) = e {
        assert_eq!(code, 404);
        assert!(message.contains("No document to update"));
        assert_eq!(context, "tests/non_existing");
    }
}
```

The code is numeric, the message is what the Google server returned as message.
The context string depends on the called method.
It may be the collection or document id or any other context information.

### Document access via service account

1. Download the service accounts credentials file and store it as "firebase-service-account.json".
   The file should contain `"private_key_id": ...`.
2. Add another field `"api_key" : "YOUR_API_KEY"` and replace YOUR_API_KEY with your *Web API key*, to be found in the [Google Firebase console](https://console.firebase.google.com) in "Project Overview -> Settings - > General".

```rust
use firestore_db_and_auth::{Credentials, ServiceSession};

/// Create credentials object. You may as well do that programmatically.
let cred = Credentials::from_file("firebase-service-account.json")
    .expect("Read credentials file");

/// To use any of the Firestore methods, you need a session first. You either want
/// an impersonated session bound to a Firebase Auth user or a service account session.
let session = ServiceSession::new(&cred)
    .expect("Create a service account session");
```

### Document access via a firebase user access / refresh token or via user_id

You can create a user session in various ways.
If you just have the firebase Auth user_id, you would follow these steps:

```rust
use firestore_db_and_auth::{Credentials, sessions};

/// Create credentials object. You may as well do that programmatically.
let cred = Credentials::from_file("firebase-service-account.json")
    .expect("Read credentials file");

/// To use any of the Firestore methods, you need a session first.
/// Create an impersonated session bound to a Firebase Auth user via your service account credentials.
let session = UserSession::by_user_id(&cred, "the_user_id")
    .expect("Create a user session");
```

If you already have a valid refresh token and want to generate an access token (and a session object), you do this instead:

```rust
let refresh_token = "fkjandsfbajsbfd;asbfdaosa.asduabsifdabsda,fd,a,sdbasfadfasfas.dasdasbfadusbflansf";
let session = UserSession::by_refresh_token(&cred, &refresh_token)?;
```

Another way of retrieving a session object is by providing a valid access token like so:

```rust
let access_token = "fkjandsfbajsbfd;asbfdaosa.asduabsifdabsda,fd,a,sdbasfadfasfas.dasdasbfadusbflansf";
let session = UserSession::by_access_token(&cred, &access_token)?;
```

The `by_access_token` method will fail if the token is not valid anymore.
Please note that a session created this way is not able to automatically refresh its access token.
(There is no *refresh_token* associated with it.)


## Cloud functions: Improve cold-start time

The usual start up procedure includes three IO operations:

* downloading two public jwks keys from a Google server,
* and read in the json credentials file.

Avoid those by embedding the credentials and public key files into your application.

First download the 2 public key files:

* https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com -> Store as `securetoken.jwks`
* https://www.googleapis.com/service_accounts/v1/jwk/{your-service-account-email} -> Store as `service-account.jwks`

Create a `Credentials` object like so:

```rust
use firestore_db_and_auth::Credentials;
let c = Credentials::new(include_str!("firebase-service-account.json"),
                         &[include_str!("securetoken.jwks"), include_str!("service-account.jwks")])?;
```

### More information

* [Firestore Auth: Background information](doc/auth_background.md)
* [Use your own authentication implementation](doc/own_auth.md)
* [Http Rocket Server integration](doc/rocket_integration.md)
* Build the documentation locally with `cargo +nightly doc --features external_doc,rocket_support`

## Testing

To perform a full integration test (`cargo test`), you need a valid "firebase-service-account.json" file.
The tests expect a Firebase user with the ID given in `tests/test_user_id.txt` to exist.
[More Information](/doc/integration_tests.md)

## Future development

Maintenance status: In Development

This library does not have the ambition to mirror the http/gRPC API 1:1.
There are auto-generated libraries for this purpose.

#### Async vs Sync

This crate uses reqwest as http client.
reqwest itself will soon have picked up full support for Rusts async+await features.

All that is left to do here then is to depend on Rust 1.39 and add an "async fn" variant that calls
and awaits reqwest for each existing method.

#### What can be done to make this crate more awesome

* Data streaming via gRPC/Protobuf
* Nice to have: Transactions, batch_get support for Firestore

