//! # Credentials for accessing the Firebase REST API
//! This module contains the [`crate::credentials::Credentials`] type, used by [`crate::sessions`] to create and maintain
//! authentication tokens for accessing the Firebase REST API.

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeMap;
use std::fs::File;
use std::sync::Arc;

use super::jwt::{create_jwt_encoded, download_google_jwks_async, verify_access_token, JWKSet, JWT_AUDIENCE_IDENTITY};
use crate::errors::FirebaseError;
use std::io::BufReader;

type Error = super::errors::FirebaseError;

/// This is not defined in the json file and computed
#[derive(Default, Clone)]
pub(crate) struct Keys {
    pub pub_key: BTreeMap<String, Arc<biscuit::jws::Secret>>,
    pub secret: Option<Arc<biscuit::jws::Secret>>,
}

/// Service account credentials
///
/// Especially the service account email is required to retrieve the public java web key set (jwks)
/// for verifying Google Firestore tokens.
///
/// The api_key is necessary for interacting with the Firestore REST API.
///
/// Internals:
///
/// The private key is used for signing JWTs (javascript web token).
/// A signed jwt, encoded as a base64 string, can be exchanged into a refresh and access token.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Credentials {
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub api_key: String,
    #[serde(default, skip)]
    pub(crate) keys: Keys,
}

/// Converts a PEM (ascii base64) encoded private key into the binary der representation
pub fn pem_to_der(pem_file_contents: &str) -> Result<Vec<u8>, Error> {
    use base64::decode;

    let pem_file_contents = pem_file_contents
        .find("-----BEGIN")
        // Cut off the first BEGIN part
        .and_then(|i| Some(&pem_file_contents[i + 10..]))
        // Find the trailing ---- after BEGIN and cut that off
        .and_then(|str| str.find("-----").and_then(|i| Some(&str[i + 5..])))
        // Cut off -----END
        .and_then(|str| str.rfind("-----END").and_then(|i| Some(&str[..i])));
    if pem_file_contents.is_none() {
        return Err(FirebaseError::Generic(
            "Invalid private key in credentials file. Must be valid PEM.",
        ));
    }

    let base64_body = pem_file_contents.unwrap().replace("\n", "");
    Ok(decode(&base64_body)
        .map_err(|_| FirebaseError::Generic("Invalid private key in credentials file. Expected Base64 data."))?)
}

#[test]
fn pem_to_der_test() {
    const INPUT: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCTbt9Rs2niyIRE
FIdrhIN757eq/1Ry/VhZALBXAveg+lt+ui/9EHtYPJH1A9NyyAwChs0UCRWqkkEo
Amtz4dJQ1YlGi0/BGhK2lg==
-----END PRIVATE KEY-----
"#;
    const EXPECTED: [u8; 112] = [
        48, 130, 4, 188, 2, 1, 0, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 4, 130, 4, 166, 48, 130, 4,
        162, 2, 1, 0, 2, 130, 1, 1, 0, 147, 110, 223, 81, 179, 105, 226, 200, 132, 68, 20, 135, 107, 132, 131, 123,
        231, 183, 170, 255, 84, 114, 253, 88, 89, 0, 176, 87, 2, 247, 160, 250, 91, 126, 186, 47, 253, 16, 123, 88, 60,
        145, 245, 3, 211, 114, 200, 12, 2, 134, 205, 20, 9, 21, 170, 146, 65, 40, 2, 107, 115, 225, 210, 80, 213, 137,
        70, 139, 79, 193, 26, 18, 182, 150,
    ];

    assert_eq!(&EXPECTED[..], &pem_to_der(INPUT).unwrap()[..]);
}

impl Credentials {
    /// Create a [`Credentials`] object by parsing a google-service-account json string
    ///
    /// Example:
    ///
    /// Assuming that your firebase service account credentials file is called "service-account-test.json" and
    /// a downloaded jwk-set file is called "service-account-test.jwks" this example embeds
    /// the file content during compile time. This avoids and http or io calls.
    ///
    /// ```
    /// use firestore_db_and_auth::{Credentials};
    /// use firestore_db_and_auth::jwt::JWKSet;
    ///
    /// let c: Credentials = Credentials::new(include_str!("../tests/service-account-test.json"))?
    ///     .with_jwkset(&JWKSet::new(include_str!("../tests/service-account-test.jwks"))?)?;
    /// # Ok::<(), firestore_db_and_auth::errors::FirebaseError>(())
    /// ```
    ///
    /// You need two JWKS files for this crate to work:
    /// * https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com
    /// * https://www.googleapis.com/service_accounts/v1/jwk/{your-service-account-email}
    pub fn new(credentials_file_content: &str) -> Result<Credentials, Error> {
        let mut credentials: Credentials = serde_json::from_str(credentials_file_content)?;
        credentials.compute_secret()?;
        Ok(credentials)
    }

    /// Create a [`Credentials`] object by reading and parsing a google-service-account json file.
    ///
    /// This is a convenience method, that reads in the given credentials file and acts otherwise the same as
    /// the [`Credentials::new`] method.
    pub fn from_file(credential_file: &str) -> Result<Self, Error> {
        let f = BufReader::new(File::open(credential_file)?);
        let mut credentials: Credentials = serde_json::from_reader(f)?;
        credentials.compute_secret()?;
        Ok(credentials)
    }

    /// Adds public-key JWKs to a credentials instance and returns it.
    ///
    /// This method will also verify that the given JWKs files allow verification of Google access tokens.
    /// This is a convenience method, you may also just use [`Credentials::add_jwks_public_keys`].
    pub fn with_jwkset(mut self, jwks: &JWKSet) -> Result<Credentials, Error> {
        self.add_jwks_public_keys(jwks);
        self.verify()?;
        Ok(self)
    }

    /// The public keys to verify generated tokens will be downloaded, for the given service account as well as
    /// for "securetoken@system.gserviceaccount.com".
    /// Do not use this option if additional downloads are not desired,
    /// for example in cloud functions that require fast cold boot start times.
    ///
    /// You can use [`Credentials::add_jwks_public_keys`] to manually add/replace public keys later on.
    ///
    /// Example:
    ///
    /// Assuming that your firebase service account credentials file is called "service-account-test.json".
    ///
    /// ```no_run
    /// use firestore_db_and_auth::{Credentials};
    ///
    /// let c: Credentials = Credentials::new(include_str!("../tests/service-account-test.json"))?
    ///     .download_jwkset()?;
    /// # Ok::<(), firestore_db_and_auth::errors::FirebaseError>(())
    /// ```
    pub async fn download_jwkset(mut self) -> Result<Credentials, Error> {
        self.download_google_jwks().await?;
        self.verify()?;
        Ok(self)
    }

    /// Verifies that creating access tokens is possible with the given credentials and public keys.
    /// Returns an empty result type on success.
    pub fn verify(&self) -> Result<(), Error> {
        let access_token = create_jwt_encoded(
            &self,
            Some(["admin"].iter()),
            std::time::Duration::from_secs(60 * 60),
            Some(self.client_id.clone()),
            None,
            JWT_AUDIENCE_IDENTITY,
        )?;
        verify_access_token(&self, &access_token)?;
        Ok(())
    }

    /// Find the secret in the jwt set that matches the given key id, if any.
    /// Used for jws validation
    pub fn decode_secret(&self, kid: &str) -> Option<Arc<biscuit::jws::Secret>> {
        self.keys.pub_key.get(kid).and_then(|f| Some(f.clone()))
    }

    /// Add a JSON Web Key Set (JWKS) to allow verification of Google access tokens.
    ///
    /// Example:
    ///
    /// ```
    /// use firestore_db_and_auth::credentials::Credentials;
    /// use firestore_db_and_auth::JWKSet;
    ///
    /// let mut c : Credentials = serde_json::from_str(include_str!("../tests/service-account-test.json"))?;
    /// c.add_jwks_public_keys(&JWKSet::new(include_str!("../tests/service-account-test.jwks"))?);
    /// c.compute_secret()?;
    /// c.verify()?;
    /// # Ok::<(), firestore_db_and_auth::errors::FirebaseError>(())
    /// ```
    pub fn add_jwks_public_keys(&mut self, jwkset: &JWKSet) {
        for entry in jwkset.keys.iter() {
            if !entry.headers.key_id.is_some() {
                continue;
            }

            let key_id = entry.headers.key_id.as_ref().unwrap().to_owned();
            self.keys
                .pub_key
                .insert(key_id, Arc::new(entry.ne.jws_public_key_secret()));
        }
    }

    /// If you haven't called [`Credentials::add_jwks_public_keys`] to manually add public keys,
    /// this method will download one for your google service account and one for the oauth related
    /// securetoken@system.gserviceaccount.com service account.
    pub async fn download_google_jwks(&mut self) -> Result<(), Error> {
        let jwks = download_google_jwks_async(&self.client_email).await?;
        self.add_jwks_public_keys(&JWKSet::new(&jwks)?);
        let jwks = download_google_jwks_async("securetoken@system.gserviceaccount.com").await?;
        self.add_jwks_public_keys(&JWKSet::new(&jwks)?);
        Ok(())
    }
    /// Compute the Rsa keypair by using the private_key of the credentials file.
    /// You must call this if you have manually created a credentials object.
    ///
    /// This is automatically invoked if you use [`Credentials::new`] or [`Credentials::from_file`].
    pub fn compute_secret(&mut self) -> Result<(), Error> {
        use biscuit::jws::Secret;
        use ring::signature;

        let vec = pem_to_der(&self.private_key)?;
        let key_pair = signature::RsaKeyPair::from_pkcs8(&vec)?;
        self.keys.secret = Some(Arc::new(Secret::RsaKeyPair(Arc::new(key_pair))));
        Ok(())
    }
}

#[doc(hidden)]
#[allow(dead_code)]
pub fn doctest_credentials() -> Credentials {
    let jwk_list = JWKSet::new(include_str!("../tests/service-account-test.jwks")).unwrap();
    Credentials::new(include_str!("../tests/service-account-test.json"))
        .expect("Failed to deserialize credentials")
        .with_jwkset(&jwk_list)
        .expect("JWK public keys verification failed")
}

#[test]
fn deserialize_credentials() {
    let jwk_list = JWKSet::new(include_str!("../tests/service-account-test.jwks")).unwrap();
    let c: Credentials = Credentials::new(include_str!("../tests/service-account-test.json"))
        .expect("Failed to deserialize credentials")
        .with_jwkset(&jwk_list)
        .expect("JWK public keys verification failed");
    assert_eq!(c.api_key, "api_key");

    use std::path::PathBuf;
    let mut credential_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    credential_file.push("tests/service-account-test.json");

    let c = Credentials::from_file(credential_file.to_str().unwrap())
        .expect("Failed to open credentials file")
        .with_jwkset(&jwk_list)
        .expect("JWK public keys verification failed");
    assert_eq!(c.api_key, "api_key");
}
