#![feature(proc_macro_hygiene, decl_macro)]

use firestore_db_and_auth::{Credentials, rocket::FirestoreAuthSessionGuard};
use rocket::{get, routes};

fn main() {
    let credentials = Credentials::from_file("firebase-service-account.json").unwrap();
    
    let config = Config::build(Environment::Staging)
        .port(8000)
        .finalize()?;

    rocket::custom(config)
        .manage(credentials)
        .mount("/", routes![hello, hello_not_logged_in])
        .launch();
}

/// Example route. Try with /hello?auth=THE_TOKEN. This works because the auth guard
/// either accepts an "Authorization: Bearer THE_TOKEN" http header or an url parameter "auth".
#[get("/hello")]
fn hello<'r>(auth: FirestoreAuthSessionGuard) -> String {
    // ApiKey is a single value tuple with a sessions::user::Session object inside
    format!("you are logged in. user_id: {}", auth.0.user_id)
}

#[get("/hello")]
fn hello_not_logged_in<'r>() -> &'r str {
    "you are not logged in"
}
