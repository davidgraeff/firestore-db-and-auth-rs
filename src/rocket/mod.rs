//! # Rocket Authentication Guard
//!
//! Because the `sessions` module of this crate is already able to verify access tokens,
//! it was not much more work to turn this into a Rocket 0.4+ Guard.
//!
//! The implemented Guard (enabled by the feature "rocket_support") allows access to http paths
//! if the provided http "Authorization" header contains a valid "Bearer" token.
//! The above mentioned validations on the token are performed.
//!
//! Example:
//!
//! ```
//! use firestore_db_and_auth::{Credentials, sessions::service_account, rocket::FirestoreAuthSessionGuard};
//!
//! fn main() {
//!     let credentials = Credentials::from_file("firebase-service-account.json").unwrap();
//!     rocket::ignite().manage(credentials).mount("/", routes![hello, hello_not_logged_in]).launch();
//! }
//!
//! /// And an example route could be:
//! #[get("/hello")]
//! fn hello<'r>(_api_key: FirestoreAuthSessionGuard,) -> &'r str {
//!     // ApiKey is a single value tuple with a sessions::user::Session object inside
//!     "you are logged in"
//! }
//!
//! #[get("/hello")]
//! fn hello_not_logged_in<'r>() -> &'r str {
//!     "you are not logged in"
//! }
//! ```
use super::credentials::Credentials;
use super::errors::FirebaseError;
use super::sessions;
use rocket::{http::Status, request, Outcome, State};

/// Use this Rocket guard to secure a route for authenticated users only.
/// Will return the associated session, that contains the used access token for further use
/// and access to the Firestore database.
pub struct FirestoreAuthSessionGuard(pub sessions::user::Session);

impl<'a, 'r> request::FromRequest<'a, 'r> for FirestoreAuthSessionGuard {
    type Error = FirebaseError;

    fn from_request(request: &'a request::Request<'r>) -> request::Outcome<Self, Self::Error> {
        let r = request
            .headers()
            .get_one("Authorization")
            .map(|f| f.to_owned())
            .or(request.get_query_value("auth").and_then(|r| r.ok()));
        if r.is_none() {
            return Outcome::Failure((Status::BadRequest, FirebaseError::Generic("")));
        }
        let db = request
            .guard::<State<Credentials>>()
            .success_or(FirebaseError::Generic(""));
        if db.is_err() {
            return Outcome::Failure((Status::BadRequest, db.err().unwrap()));
        }
        let bearer = r.unwrap();
        if !bearer.starts_with("Bearer ") {
            return Outcome::Failure((
                Status::BadRequest,
                FirebaseError::Generic("Only bearer authorization accepted"),
            ));
        }
        let bearer = &bearer[7..];
        let session = sessions::user::Session::by_access_token(&db.unwrap(), bearer);
        if session.is_err() {
            return Outcome::Failure((Status::Unauthorized, session.err().unwrap()));
        }
        Outcome::Success(FirestoreAuthSessionGuard(session.unwrap()))
    }
}
