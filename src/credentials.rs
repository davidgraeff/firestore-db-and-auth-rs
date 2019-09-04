//! # Credentials for accessing the Firebase REST API
//! This module contains the [`crate::credentials::Credentials`] type, used by [`crate::sessions`] to create and maintain
//! authentication tokens for accessing the Firebase REST API.

use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;

use super::jwt::{
    create_jwt_encoded, download_google_jwks, verify_access_token, JWKSetDTO, JWT_AUDIENCE_IDENTITY,
};

type Error = super::errors::FirebaseError;

/// This is not defined in the json file and computed
#[derive(Default)]
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

impl Clone for Keys {
    fn clone(&self) -> Self {
        Self {
            pub_key: Default::default(),
            secret: None,
        }
    }
}

/// Converts a PEM (ascii base64) encoded private key into the binary der representation
pub fn pem_to_der(pem_file_contents: &str) -> Vec<u8> {
    use regex::Regex;
    use rustc_serialize::base64::FromBase64;
    const REGEX: &'static str = r"(-----BEGIN .*-----\n)((?:(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)*\n)+)(-----END .*-----)";

    let re = Regex::new(REGEX).unwrap();
    let contents_without_headers = re.replace(pem_file_contents, "$2");
    let base64_body = contents_without_headers.replace("\n", "");
    base64_body.from_base64().unwrap()
}

impl Credentials {
    /// Create a [`Credentials`] object by parsing a google-service-account json string
    /// and public-key JWKs strings.
    ///
    /// This method will also verify that the given JWKs files are matching
    ///
    /// Example:
    ///
    /// Assuming that your credentials file is called "firebase-service-account.json" and
    /// a downloaded jwk-set file is called "service-account-for-tests.jwks" this example embeds
    /// the file content during compile time. This avoids and http or io calls.
    ///
    /// ```
    /// use firestore_db_and_auth::credentials::Credentials;
    ///
    /// let c : Credentials = Credentials::new(include_str!("../firebase-service-account.json"),
    ///                                         &[include_str!("../tests/service-account-for-tests.jwks")])?;
    /// # Ok::<(), firestore_db_and_auth::errors::FirebaseError>(())
    /// ```
    ///
    /// You need two JWKS files for this crate to work:
    /// * https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com
    /// * https://www.googleapis.com/service_accounts/v1/jwk/{your-service-account-email}
    pub fn new(credentials_file_content: &str, jwks_files: &[&str]) -> Result<Credentials, Error> {
        let mut credentials: Credentials = serde_json::from_str(credentials_file_content)?;
        for jwks_file in jwks_files {
            credentials.add_jwks_public_keys(serde_json::from_str(jwks_file)?);
        }
        credentials.compute_secret()?;
        credentials.verify()?;
        Ok(credentials)
    }

    pub fn verify(&self) -> Result<(), Error> {
        let access_token = create_jwt_encoded(
            &self,
            Some(["admin"].iter()),
            Duration::hours(1),
            Some(self.client_id.clone()),
            None,
            JWT_AUDIENCE_IDENTITY,
        )?;
        verify_access_token(&self, &access_token)?.ok_or(Error::Generic(
            "Verification failed. Credentials do not match the public keys",
        ))?;
        Ok(())
    }

    /// Create a [`Credentials`] object by reading and parsing a google-service-account json file.
    ///
    /// The public keys to verify generated tokens will be downloaded, for the given service account as well as
    /// for "securetoken@system.gserviceaccount.com".
    ///
    /// Do not use this method if this is not desired, for example in cloud functions that require fast cold start times.
    /// See [`Credentials::add_jwks_public_keys`] and [`Credentials::new`] as alternatives.
    pub fn from_file(credential_file: &str) -> Result<Self, Error> {
        let mut f = File::open(credential_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let mut credentials: Credentials = serde_json::from_slice(buffer.as_slice())?;
        credentials.compute_secret()?;
        credentials.download_google_jwks()?;
        Ok(credentials)
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
    ///
    /// let mut c : Credentials = serde_json::from_str(include_str!("../firebase-service-account.json")).unwrap();
    /// c.add_jwks_public_keys(serde_json::from_str(include_str!("../tests/service-account-for-tests.jwks")).unwrap());
    /// c.compute_secret().unwrap();
    /// ```
    pub fn add_jwks_public_keys(&mut self, jwkset: JWKSetDTO) {
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

    /// Compute the Rsa keypair by using the private_key of the credentials file.
    /// You must call this if you have manually created a credentials object.
    ///
    /// This is automatically invoked if you use [`Credentials::new`] or [`Credentials::from_file`].
    pub fn compute_secret(&mut self) -> Result<(), Error> {
        use biscuit::jws::Secret;
        use ring::signature;

        let vec = pem_to_der(&self.private_key);
        let key_pair = signature::RsaKeyPair::from_pkcs8(&vec)?;
        self.keys.secret = Some(Arc::new(Secret::RsaKeyPair(Arc::new(key_pair))));
        Ok(())
    }

    /// If you haven't called [`Credentials::add_jwks_public_keys`] to manually add public keys,
    /// this method will download one for your google service account and one for the oauth related
    /// securetoken@system.gserviceaccount.com service account.
    pub fn download_google_jwks(&mut self) -> Result<(), Error> {
        if self.keys.pub_key.is_empty() {
            let jwks = download_google_jwks(&self.client_email)?;
            self.add_jwks_public_keys(jwks);
            let jwks = download_google_jwks("securetoken@system.gserviceaccount.com")?;
            self.add_jwks_public_keys(jwks);
        }
        Ok(())
    }
}
