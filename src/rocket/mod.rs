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
//! use firestore_db_and_auth::{Credentials, rocket::FirestoreAuthSessionGuard};
//!
//! fn main() {
//!     let credentials = Credentials::from_file("firebase-service-account.json").unwrap();
//!     rocket::ignite().manage(credentials).mount("/", routes![hello, hello_not_logged_in]).launch();
//! }
//!
//! /// And an example route could be:
//! #[get("/hello")]
//! fn hello<'r>(auth: FirestoreAuthSessionGuard) -> String {
//!     // ApiKey is a single value tuple with a sessions::user::Session object inside
//!    format!("you are logged in. user_id: {}", auth.0.user_id)
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
pub struct FirestoreAuthSessionGuard(pub sessions::user::BlockingSession);

impl<'a, 'r> request::FromRequest<'a, 'r> for FirestoreAuthSessionGuard {
    type Error = FirebaseError;

    fn from_request(request: &'a request::Request<'r>) -> request::Outcome<Self, Self::Error> {
        let r = request
            .headers()
            .get_one("Authorization")
            .map(|f| f.to_owned())
            .or(request.get_query_value("auth").and_then(|r| r.ok()));
        if r.is_none() {
            return Outcome::Forward(());
        }
        let bearer = r.unwrap();
        if !bearer.starts_with("Bearer ") {
            return Outcome::Forward(());
        }
        let bearer = &bearer[7..];

        // You MUST make the credentials object available as managed state to rocket!
        let db = match request.guard::<State<Credentials>>() {
            Outcome::Success(db) => db,
            _ => {
                return Outcome::Failure((
                    Status::InternalServerError,
                    FirebaseError::Generic("Firestore credentials not set!"),
                ))
            }
        };

        let session = sessions::user::BlockingSession::by_access_token(&db, bearer);
        if session.is_err() {
            return Outcome::Forward(());
        }
        Outcome::Success(FirestoreAuthSessionGuard(session.unwrap()))
    }
}
