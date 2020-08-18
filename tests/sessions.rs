use firestore_db_and_auth::*;
use firestore_db_and_auth::sessions::user::Session;
use std::fs;

#[test]
fn user_session_create_session_cookie() -> errors::Result<()> {
    let cred = credentials::Credentials::from_file("firebase-service-account.json").expect("Read credentials file");
    let id_token: String = fs::read_to_string("refresh-token-for-tests.txt")?;

    println!("users::Session::create_session_cookie");
    Session::create_session_cookie(&cred, id_token.to_string(), 3600)?;

    Ok(())
}
