//! # A Firestore Auth Session token is a Javascript Web Token (JWT). This module contains JWT helper functions.

use super::credentials::Credentials;

use serde::{Deserialize, Serialize};

use chrono::{Duration, Utc};
use std::collections::HashSet;
use std::slice::Iter;
use std::ops::Add;

use crate::errors::FirebaseError;
use biscuit::jwa::SignatureAlgorithm;
use biscuit::{ClaimPresenceOptions, SingleOrMultiple, ValidationOptions};
use std::ops::Deref;
use cache_control::CacheControl;

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
pub async fn download_google_jwks(account_mail: &str) -> Result<(String, Option<Duration>), Error> {
    let url = format!("https://www.googleapis.com/service_accounts/v1/jwk/{}", account_mail);
    let resp = reqwest::Client::new().get(&url).send().await?;
    let max_age = resp.headers()
        .get("cache-control")
        .and_then(|cache_control| cache_control.to_str().ok())
        .and_then(|cache_control| CacheControl::from_value(cache_control))
        .and_then(|cache_control| cache_control.max_age)
        .and_then(|max_age| Duration::from_std(max_age).ok());

    Ok((resp.text().await?, max_age))
}

pub(crate) async fn create_jwt_encoded<S: AsRef<str>>(
    credentials: &Credentials,
    scope: Option<Iter<'_, S>>,
    duration: chrono::Duration,
    client_id: Option<String>,
    user_id: Option<String>,
    audience: &str,
) -> Result<String, Error> {
    let jwt = create_jwt(credentials, scope, duration, client_id, user_id, audience)?;
    let secret_lock = credentials
        .keys
        .lock()
        .await;
    let secret = secret_lock
        .secret
        .as_ref()
        .ok_or(Error::Generic("No private key added via add_keypair_key!"))?;
    Ok(jwt.encode(&secret.deref())?.encoded()?.encode())
}

/// Returns true if the access token (assumed to be a jwt) has expired
///
/// An error is returned if the given access token string is not a jwt
pub(crate) fn is_expired(access_token: &str, tolerance_in_minutes: i64) -> Result<bool, FirebaseError> {
    let token = AuthClaimsJWT::new_encoded(&access_token);
    let claims = token.unverified_payload()?;
    if let Some(expiry) = claims.registered.expiry.as_ref() {
        let diff: Duration = Utc::now().signed_duration_since(expiry.deref().clone());
        return Ok(diff.num_minutes() - tolerance_in_minutes > 0);
    }

    Ok(true)
}

/// Returns true if the jwt was updated and needs signing
pub(crate) fn jwt_update_expiry_if(jwt: &mut AuthClaimsJWT, expire_in_minutes: i64) -> bool {
    let ref mut claims = jwt.payload_mut().unwrap().registered;

    let now = biscuit::Timestamp::from(Utc::now());
    let now_plus_hour = biscuit::Timestamp::from(Utc::now().add(Duration::hours(1)));

    if let Some(issued_at) = claims.issued_at.as_ref() {
        let diff: Duration = Utc::now().signed_duration_since(issued_at.deref().clone());
        if diff.num_minutes() > expire_in_minutes {
            claims.issued_at = Some(now);
            claims.expiry = Some(now_plus_hour);
        } else {
            return false;
        }
    } else {
        claims.issued_at = Some(now);
        claims.expiry = Some(now_plus_hour);
    }

    true
}

pub(crate) fn create_jwt<S>(
    credentials: &Credentials,
    scope: Option<Iter<S>>,
    duration: chrono::Duration,
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
            expiry: Some(biscuit::Timestamp::from(Utc::now().add(duration))),
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

#[derive(Debug)]
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

pub(crate) async fn verify_access_token(
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
        .await?
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

pub mod session_cookie {
    use super::*;
    use std::ops::Add;

    pub(crate) async fn create_jwt_encoded(credentials: &Credentials, duration: chrono::Duration) -> Result<String, Error> {
        let scope = [
            "https://www.googleapis.com/auth/cloud-platform",
            "https://www.googleapis.com/auth/firebase.database",
            "https://www.googleapis.com/auth/firebase.messaging",
            "https://www.googleapis.com/auth/identitytoolkit",
            "https://www.googleapis.com/auth/userinfo.email",
        ];

        const AUDIENCE: &str = "https://accounts.google.com/o/oauth2/token";

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
                audience: Some(SingleOrMultiple::Single(AUDIENCE.to_string())),
                subject: Some(credentials.client_email.clone()),
                expiry: Some(biscuit::Timestamp::from(Utc::now().add(duration))),
                issued_at: Some(biscuit::Timestamp::from(Utc::now())),
                ..Default::default()
            },
            private: JwtOAuthPrivateClaims {
                scope: Some(scope.join(" ")),
                client_id: None,
                uid: None,
            },
        };
        let jwt = JWT::new_decoded(header, expected_claims);

        let secret_lock = credentials
            .keys
            .lock()
            .await;
        let secret = secret_lock
            .secret
            .as_ref()
            .ok_or(Error::Generic("No private key added via add_keypair_key!"))?;
        Ok(jwt.encode(&secret.deref())?.encoded()?.encode())
    }
}
