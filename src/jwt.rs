//! # A Firestore Auth Session token is a Javascript Web Token (JWT). This module contains JWT helper functions.

use super::credentials::Credentials;

use serde::{Deserialize, Serialize};

use chrono::{Utc};
use std::collections::HashSet;
use std::slice::Iter;

use crate::errors::FirebaseError;
use biscuit::jwa::SignatureAlgorithm;
use biscuit::{ClaimPresenceOptions, SingleOrMultiple, ValidationOptions};
use std::ops::{Deref, Add};

type Error = super::errors::FirebaseError;

pub static JWT_AUDIENCE_FIRESTORE: &str = "https://firestore.googleapis.com/google.firestore.v1.Firestore";
pub static JWT_AUDIENCE_IDENTITY: &str =
    "https://identitytoolkit.googleapis.com/google.identity.identitytoolkit.v1.IdentityToolkit";

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct JwtOAuthPrivateClaims {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>, // Probably the firebase User ID if set
}

pub(crate) type AuthClaimsJWT = biscuit::JWT<JwtOAuthPrivateClaims, biscuit::Empty>;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct JWSEntry {
    #[serde(flatten)]
    pub(crate) headers: biscuit::jws::RegisteredHeader,
    #[serde(flatten)]
    pub(crate) ne: biscuit::jwk::RSAKeyParameters,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct JWKSet {
    pub keys: Vec<JWSEntry>,
}

impl JWKSet {
    /// Create a new JWKSetDTO instance from a given json string
    /// You can use [`Credentials::add_jwks_public_keys`] to manually add more public keys later on.
    /// You need two JWKS files for this crate to work:
    /// * https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com
    /// * https://www.googleapis.com/service_accounts/v1/jwk/{your-service-account-email}
    pub fn new(jwk_content: &str) -> Result<JWKSet, Error> {
        let jwk_set: JWKSet = serde_json::from_str(jwk_content).map_err(|e| FirebaseError::Ser {
            doc: Option::from(format!("Failed to parse jwkset. Return value: {}", jwk_content)),
            ser: e,
        })?;
        Ok(jwk_set)
    }
}

/// Download the Google JWK Set for a given service account.
/// The resulting set of JWKs need to be added to a credentials object
/// for jwk verifications.
pub async fn download_google_jwks_async(account_mail: &str) -> Result<String, Error> {
    let resp = reqwest::Client::new()
        .get(&format!(
            "https://www.googleapis.com/service_accounts/v1/jwk/{}",
            account_mail
        ))
        .send()
        .await?;
    Ok(resp.text().await?)
}

pub(crate) fn create_jwt_encoded<S: AsRef<str>>(
    credentials: &Credentials,
    scope: Option<Iter<S>>,
    duration: std::time::Duration,
    client_id: Option<String>,
    user_id: Option<String>,
    audience: &str,
) -> Result<String, Error> {
    let jwt = create_jwt(credentials, scope, duration, client_id, user_id, audience)?;
    let secret = credentials
        .keys
        .secret
        .as_ref()
        .ok_or(Error::Generic("No private key added via add_keypair_key!"))?;
    Ok(jwt.encode(&secret.deref())?.encoded()?.encode())
}

/// Returns true if the access token (assumed to be a jwt) has expired
///
/// An error is returned if the given access token string is not a jwt
pub(crate) fn expires(access_token: &str) -> Result<std::time::Duration, FirebaseError> {
    expires_jwt(&AuthClaimsJWT::new_encoded(&access_token))
}

pub(crate) fn expires_jwt(jwt: &AuthClaimsJWT) -> Result<std::time::Duration, FirebaseError> {
    let claims = jwt.unverified_payload()?;
    if let Some(expiry) = claims.registered.expiry.as_ref() {
        return Ok(Utc::now().signed_duration_since(expiry.deref().clone()).to_std()
            // A negative value means the token has expired, just return 0
            .map_err(|_| std::time::Duration::from_secs(0)).unwrap());
    }

    Ok(std::time::Duration::from_secs(0))
}


/// Updates the issued_at and expiry fields of the given jwt and return the signed jwt
pub(crate) fn jwt_update_expiry_and_sign(jwt: &mut AuthClaimsJWT, secret: &biscuit::jws::Secret, expire_time: std::time::Duration) -> Result<String, FirebaseError> {
    let ref mut claims = jwt.payload_mut().unwrap().registered;
    claims.issued_at = Some(biscuit::Timestamp::from(Utc::now()));
    let duration = chrono::Duration::from_std(expire_time).map_err(|_| chrono::Duration::hours(1)).unwrap();
    claims.expiry = Some(biscuit::Timestamp::from(Utc::now().add(duration)));
    let encoded = jwt.encode(&secret.deref()).map_err(|e| FirebaseError::JWT(e))?;
    Ok(encoded.encoded().map_err(|e| FirebaseError::JWT(e))?.encode())
}

pub(crate) fn create_jwt<S>(
    credentials: &Credentials,
    scope: Option<Iter<S>>,
    duration: std::time::Duration,
    client_id: Option<String>,
    user_id: Option<String>,
    audience: &str,
) -> Result<AuthClaimsJWT, Error>
    where
        S: AsRef<str>,
{
    use biscuit::{
        jws::{Header, RegisteredHeader},
        ClaimsSet, Empty, RegisteredClaims, JWT,
    };

    let header: Header<Empty> = Header::from(RegisteredHeader {
        algorithm: SignatureAlgorithm::RS256,
        key_id: Some(credentials.private_key_id.to_owned()),
        ..Default::default()
    });
    let expected_claims = ClaimsSet::<JwtOAuthPrivateClaims> {
        registered: RegisteredClaims {
            issuer: Some(credentials.client_email.clone()),
            audience: Some(SingleOrMultiple::Single(audience.to_string())),
            subject: Some(credentials.client_email.clone()),
            expiry: Some(biscuit::Timestamp::from(Utc::now().add( chrono::Duration::from_std(duration).unwrap()))),
            issued_at: Some(biscuit::Timestamp::from(Utc::now())),
            ..Default::default()
        },
        private: JwtOAuthPrivateClaims {
            scope: scope.and_then(|f| {
                Some(f.fold(String::new(), |acc, x| {
                    let x: &str = x.as_ref();
                    return acc + x + " ";
                }))
            }),
            client_id,
            uid: user_id,
        },
    };
    Ok(JWT::new_decoded(header, expected_claims))
}

pub struct TokenValidationResult {
    pub claims: JwtOAuthPrivateClaims,
    pub audience: String,
    pub subject: String,
}

impl TokenValidationResult {
    pub fn get_scopes(&self) -> HashSet<String> {
        match self.claims.scope {
            Some(ref v) => v.split(" ").map(|f| f.to_owned()).collect(),
            None => HashSet::new(),
        }
    }
}

pub(crate) fn verify_access_token(
    credentials: &Credentials,
    access_token: &str,
) -> Result<TokenValidationResult, Error> {
    let token = AuthClaimsJWT::new_encoded(&access_token);

    let header = token.unverified_header()?;
    let kid = header
        .registered
        .key_id
        .as_ref()
        .ok_or(FirebaseError::Generic("No jwt kid"))?;
    let secret = credentials
        .decode_secret(kid)
        .ok_or(FirebaseError::Generic("No secret for kid"))?;

    let token = token.into_decoded(&secret.deref(), SignatureAlgorithm::RS256)?;

    use biscuit::Presence::*;

    let o = ValidationOptions {
        claim_presence_options: ClaimPresenceOptions {
            issued_at: Required,
            not_before: Optional,
            expiry: Required,
            issuer: Required,
            audience: Required,
            subject: Required,
            id: Optional,
        },
        // audience: Validation::Validate(StringOrUri::from_str(JWT_SUBJECT)?),
        ..Default::default()
    };

    let claims = token.payload()?;
    claims.registered.validate(o)?;

    let audience = match claims.registered.audience.as_ref().unwrap() {
        SingleOrMultiple::Single(v) => v.to_string(),
        SingleOrMultiple::Multiple(v) => v.get(0).unwrap().to_string(),
    };

    Ok(TokenValidationResult {
        claims: claims.private.clone(),
        subject: claims.registered.subject.as_ref().unwrap().to_string(),
        audience,
    })
}
