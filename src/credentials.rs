//! # Credentials for accessing the Firebase REST API
//! This module contains the [`crate::credentials::Credentials`] type, used by [`crate::sessions`] to create and maintain
//! authentication tokens for accessing the Firebase REST API.

use std::fs::File;
use std::io::prelude::*;

use super::errors;
use serde::{Deserialize, Serialize};
use serde_json;

use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct JWSEntry {
    #[serde(flatten)]
    headers: biscuit::jws::RegisteredHeader,
    #[serde(flatten)]
    ne: biscuit::jwk::RSAKeyParameters,
}

#[derive(Serialize, Deserialize)]
pub struct JWKSetDTO {
    pub keys: Vec<JWSEntry>,
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
    private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub api_key: String,
    #[serde(default, skip)]
    /// This is not defined in the json file and computed
    pub pub_key: BTreeMap<String, biscuit::jwk::RSAKeyParameters>,
    #[serde(default)]
    /// This is not defined in the json file and computed
    pub private_key_der: Vec<u8>,
}

use regex::Regex;
use rustc_serialize::base64::FromBase64;
const REGEX: &'static str = r"(-----BEGIN .*-----\n)((?:(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)*\n)+)(-----END .*-----)";

fn pem_to_der(pem_file_contents: &str) -> Vec<u8> {
    let re = Regex::new(REGEX).unwrap();
    let contents_without_headers = re.replace(pem_file_contents, "$2");
    let base64_body = contents_without_headers.replace("\n", "");
    base64_body.from_base64().unwrap()
}

fn download_google_jwks(account_mail: &str) -> errors::Result<JWKSetDTO> {
    let mut resp = reqwest::Client::new()
        .get(&format!(
            "https://www.googleapis.com/service_accounts/v1/jwk/{}",
            account_mail
        ))
        .send()?;
    let jwkset: JWKSetDTO = resp.json()?;
    Ok(jwkset)
}

impl Credentials {
    /// Find the key in the set that matches the given key id, if any.
    pub fn public_key(&self, kid: &str) -> Option<&biscuit::jwk::RSAKeyParameters> {
        self.pub_key.get(kid)
    }

    /// Creates a ring RsaKeyPair out of the private key of this credentials object
    pub fn create_rsa_key_pair(&self) -> errors::Result<ring::signature::RsaKeyPair> {
        ring::signature::RsaKeyPair::from_pkcs8(&self.private_key_der).map_err(|e| e.into())
    }

    /// Create a [`Credentials`] object by reading and parsing a google-service-account json file.
    ///
    /// The public keys to verify generated tokens will be downloaded, for the given service account as well as
    /// for "securetoken@system.gserviceaccount.com".
    /// Do not use this method if this is not desired, see [`Credentials::add_jwks_public_keys`] for an alternative.
    pub fn from_file(credential_file: &str) -> errors::Result<Self> {
        let mut f = File::open(credential_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let mut credentials: Credentials = serde_json::from_slice(buffer.as_slice())?;
        credentials.compute_missing_fields()?;
        Ok(credentials)
    }

    /// Add a jwks file to verify Google access tokens.
    ///
    /// Example:
    ///
    /// ```
    /// use firestore_db_and_auth::credentials::Credentials;
    ///
    /// let mut c : Credentials = serde_json::from_str(include_str!("../firebase-service-account.json")).unwrap();
    /// c.add_jwks_public_keys(serde_json::from_str(include_str!("../service-account-for-tests.jwks")).unwrap());
    /// c.compute_missing_fields().unwrap();
    /// ```
    pub fn add_jwks_public_keys(&mut self, mut jwkset: JWKSetDTO) {
        for entry in jwkset.keys.drain(..) {
            if !entry.headers.key_id.is_some() {
                continue;
            }

            self.pub_key
                .insert(entry.headers.key_id.as_ref().unwrap().to_owned(), entry.ne);
        }
    }

    /// This method will compute an alternative representation of the private key and must be
    /// called before using any of the session methods.
    /// This is automatically invoked if you use [`Credentials::from_file`].
    ///
    /// If you haven't called [`Credentials::add_jwks_public_keys`] to manually add public keys,
    /// this method will download one for your google service account and one for the oauth related
    /// securetoken@system.gserviceaccount.com service account.
    pub fn compute_missing_fields(&mut self) -> errors::Result<()> {
        self.private_key_der = pem_to_der(&self.private_key);
        if self.pub_key.is_empty() {
            let jwks = download_google_jwks(&self.client_email)?;
            self.add_jwks_public_keys(jwks);
            let jwks = download_google_jwks("securetoken@system.gserviceaccount.com")?;
            self.add_jwks_public_keys(jwks);
        }
        Ok(())
    }
}

impl FromStr for Credentials {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(s)?)
    }
}
