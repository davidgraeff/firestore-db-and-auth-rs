use super::credentials::Credentials;
use super::errors::FirebaseError;
use super::sessions;
use rocket::{http::Status, request, Outcome, State};

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
