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
