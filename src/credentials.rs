
use std::fs::File;
use std::io::prelude::*;

use super::errors;
use serde::{Deserialize, Serialize};
use serde_json;

use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Default, Clone)]
struct JWSEntry {
    #[serde(flatten)]
    headers: biscuit::jws::RegisteredHeader,
    #[serde(flatten)]
    ne: biscuit::jwk::RSAKeyParameters,
}

#[derive(Serialize, Deserialize)]
struct JWKSetDTO {
    pub keys: Vec<JWSEntry>,
}

/// Service account credentials
///
/// Especially the service account email is required to retrieve the public java web key set (jwks)
/// for verifying Google Firestore tokens.
///
/// The api_key is necessary for interacting with the Firestore REST API.
///
/// The private key is used for signing java web tokens (jwk).
/// Those can be exchanged into a refresh and access token.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Credentials {
    pub project_id: String,
    pub private_key_id: String,
    private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub api_key: String,
    #[serde(default,skip)] /// This is not defined in the json file and computed
    pub pub_key: BTreeMap<String, biscuit::jwk::RSAKeyParameters>,
    #[serde(default)] /// This is not defined in the json file and computed
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

fn retrieve_jwks_for_google_account(
    result: &mut BTreeMap<String, biscuit::jwk::RSAKeyParameters>,
    account_mail: &str,
) -> errors::Result<()> {
    let mut resp = reqwest::Client::new()
        .get(&format!(
            "https://www.googleapis.com/service_accounts/v1/jwk/{}",
            account_mail
        ))
        .send()?;
    let mut jwkset: JWKSetDTO = resp.json()?;

    for entry in jwkset.keys.drain(..) {
        if !entry.headers.key_id.is_some() {
            continue;
        }

        result.insert(entry.headers.key_id.as_ref().unwrap().to_owned(), entry.ne);
    }
    Ok(())
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

    /// Create a Crentials object of a google-service-account json file.
    ///
    /// The corresponding jwks (public keys) for the given service account as well as the
    /// "securetoken@system.gserviceaccount.com" are also fetched. Those public keys
    /// are used to verify access tokens.
    pub fn from_file(credential_file: &str) -> errors::Result<Self> {
        let mut f = File::open(credential_file)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let mut credentials: Credentials = serde_json::from_slice(buffer.as_slice())?;
        credentials.compute_missing_fields()?;
        Ok(credentials)
    }

    /// Call this method after deserializing
    pub fn compute_missing_fields(&mut self) -> errors::Result<()> {
        self.private_key_der = pem_to_der(&self.private_key);
        if self.pub_key.is_empty() {
            retrieve_jwks_for_google_account(&mut self.pub_key, &self.client_email)?;
            retrieve_jwks_for_google_account(
                &mut self.pub_key,
                "securetoken@system.gserviceaccount.com",
            )?;
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