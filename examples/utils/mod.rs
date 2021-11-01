use firestore_db_and_auth::{errors, sessions, Credentials, JWKSet};

use firestore_db_and_auth::jwt::download_google_jwks;

#[allow(dead_code)]
pub const TEST_USER_ID: &str = include_str!("../test_user_id.txt");

pub async fn user_session_with_cached_refresh_token(cred: &Credentials) -> errors::Result<sessions::user::Session> {
    println!("Refresh token from file");
    // Read refresh token from file if possible instead of generating a new refresh token each time
    let refresh_token: String = match std::fs::read_to_string("refresh-token-for-tests.txt") {
        Ok(v) => v,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(errors::FirebaseError::IO(e));
            }
            String::new()
        }
    };

    // Generate a new refresh token if necessary
    println!("Generate new user auth token");
    let user_session: sessions::user::Session = if refresh_token.is_empty() {
        let session = sessions::user::Session::by_user_id(
            &cred,
            TEST_USER_ID,
            true
        ).await?;
        std::fs::write("refresh-token-for-tests.txt", &session.refresh_token.as_ref().unwrap())?;
        session
    } else {
        println!("user::Session::by_refresh_token");
        sessions::user::Session::by_refresh_token(&cred, &refresh_token).await?
    };

    Ok(user_session)
}

/// Download the two public key JWKS files if necessary and cache the content at the given file path.
/// Only use this option in cloud functions if the given file path is persistent.
/// You can use [`Credentials::add_jwks_public_keys`] to manually add more public keys later on.
pub async fn from_cache_file(cache_file: &std::path::Path, c: &Credentials) -> errors::Result<JWKSet> {
    use std::fs::File;
    use std::io::BufReader;

    Ok(if cache_file.exists() {
        let f = BufReader::new(File::open(cache_file)?);
        let jwks_set: JWKSet = serde_json::from_reader(f)?;
        jwks_set
    } else {
        // If not present, download the two jwks (specific service account + google system account),
        // merge them into one set of keys and store them in the cache file.
        let mut jwks = JWKSet::new(&download_google_jwks(&c.client_email).await?)?;
        jwks.keys
            .append(&mut JWKSet::new(&download_google_jwks("securetoken@system.gserviceaccount.com").await?)?.keys);
        let f = File::create(cache_file)?;
        serde_json::to_writer_pretty(f, &jwks)?;
        jwks
    })
}

/// For integration tests and doc code snippets: Create a Credentials instance.
/// Necessary public jwk sets are downloaded or re-used if already present.
#[cfg(test)]
#[allow(dead_code)]
pub async fn valid_test_credentials() -> errors::Result<Credentials> {
    use std::path::PathBuf;
    let mut jwks_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    jwks_path.push("firebase-service-account.jwks");

    let mut cred: Credentials = Credentials::new(include_str!("../../tests/service-account-test.json"))?;

    // Only download the public keys once, and cache them.
    let jwkset = from_cache_file(jwks_path.as_path(), &cred).await?;
    cred.add_jwks_public_keys(&jwkset);
    cred.verify()?;

    Ok(cred)
}
