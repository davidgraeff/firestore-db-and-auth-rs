#![feature(proc_macro_hygiene, decl_macro)]

use firestore_db_and_auth::{Credentials, rocket::FirestoreAuthSessionGuard};
use rocket::{get, routes};

fn main() {
    let credentials = Credentials::from_file("firebase-service-account.json").unwrap();
    rocket::ignite().manage(credentials).mount("/", routes![hello, hello_not_logged_in]).launch();
}

/// And an example route could be:
/// Try with /hello?auth=THE_TOKEN
#[get("/hello")]
fn hello<'r>(auth: FirestoreAuthSessionGuard) -> String {
    // ApiKey is a single value tuple with a sessions::user::Session object inside
    format!("you are logged in. user_id: {}", auth.0.user_id)
}

#[get("/hello")]
fn hello_not_logged_in<'r>() -> &'r str {
    "you are not logged in"
}