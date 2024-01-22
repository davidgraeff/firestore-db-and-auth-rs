use firestore_db_and_auth::{errors::FirebaseError, sessions::session_cookie, Credentials, FirebaseAuthBearer};

use chrono::Duration;

mod utils;

#[tokio::main]
async fn main() -> Result<(), FirebaseError> {
    // Search for a credentials file in the root directory
    use std::path::PathBuf;

    let mut credential_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    credential_file.push("firebase-service-account.json");
    let cred = Credentials::from_file(credential_file.to_str().unwrap()).await?;

    // Only download the public keys once, and cache them.
    let jwkset = utils::from_cache_file(credential_file.with_file_name("cached_jwks.jwks").as_path(), &cred).await?;
    cred.add_jwks_public_keys(&jwkset).await;
    cred.verify().await?;

    let user_session = utils::user_session_with_cached_refresh_token(&cred).await?;

    let cookie = session_cookie::create(&cred, user_session.access_token().await, Duration::seconds(3600)).await?;
    println!("Created session cookie: {}", cookie);

    Ok(())
}

#[test]
fn create_session_cookie_test() -> Result<(), FirebaseError> {
    let cred = utils::valid_test_credentials()?;
    let user_session = utils::user_session_with_cached_refresh_token(&cred)?;

    assert_eq!(user_session.user_id, utils::TEST_USER_ID);
    assert_eq!(user_session.project_id(), cred.project_id);

    use chrono::Duration;
    let cookie = session_cookie::create(&cred, user_session.access_token(), Duration::seconds(3600))?;

    assert!(cookie.len() > 0);
    Ok(())
}
