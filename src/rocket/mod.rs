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
//! use rocket::get;
//!
//! fn main() {
//!     use rocket::routes;
//! let credentials = Credentials::from_file("firebase-service-account.json").unwrap();
//!     rocket::build().ignite().manage(credentials).mount("/", routes![hello, hello_not_logged_in]).launch();
//! }
//!
//! /// And an example route could be:
//! #[get("/hello")]
//! fn hello<'r>(auth: FirestoreAuthSessionGuard) -> String {
//!     // ApiKey is a single value tuple with a sessions::user::Session object inside
//! format!("you are logged in. user_id: {}", auth.0.user_id)
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
use rocket::request::Outcome;
use rocket::{http::Status, request, State};

/// Use this Rocket guard to secure a route for authenticated users only.
/// Will return the associated session, that contains the used access token for further use
/// and access to the Firestore database.
pub struct FirestoreAuthSessionGuard(pub sessions::user::Session);

#[rocket::async_trait]
impl<'a> request::FromRequest<'a> for FirestoreAuthSessionGuard {
    type Error = FirebaseError;

    async fn from_request(request: &'a request::Request<'_>) -> request::Outcome<Self, Self::Error> {
        let r = request
            .headers()
            .get_one("Authorization")
            .map(|f| f.to_owned())
            .or(request.query_value("auth").and_then(|r| r.ok()));
        if r.is_none() {
            return Outcome::Forward(Status::BadRequest);
        }
        let bearer = r.unwrap();
        if !bearer.starts_with("Bearer ") {
            return Outcome::Forward(Status::BadRequest);
        }
        let bearer = &bearer[7..];

        // You MUST make the credentials object available as managed state to rocket!
        let db = match request.guard::<&State<Credentials>>().await {
            Outcome::Success(db) => db,
            _ => {
                return Outcome::Error((
                    Status::InternalServerError,
                    FirebaseError::Generic("Firestore credentials not set!"),
                ))
            }
        };

        let session = sessions::user::Session::by_access_token(&db, bearer).await;
        if session.is_err() {
            return Outcome::Forward(Status::BadRequest);
        }
        Outcome::Success(FirestoreAuthSessionGuard(session.unwrap()))
    }
}
