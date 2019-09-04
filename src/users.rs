//! # Firebase Auth API - User information
//!
//! Retrieve firebase user information

use super::errors::{extract_google_api_error, Result};

use super::sessions::user;
use serde::{Deserialize, Serialize};

use crate::FirebaseAuthBearer;
use reqwest::Client;

pub trait DocumentPath<'a> {
    fn path(&self) -> &'a str;
}

/// A federated services like Facebook, Github etc that the user has used to
/// authenticated himself and that he associated with this firebase auth account.
#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProviderUserInfo {
    pub providerId: String,
    pub federatedId: String,
    pub displayName: Option<String>,
    pub photoUrl: Option<String>,
}

/// Users id, email, display name and a few more information
#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FirebaseAuthUser {
    pub localId: Option<String>,
    pub email: Option<String>,
    /// True if the user has verified his email address
    pub emailVerified: Option<bool>,
    pub displayName: Option<String>,
    /// Find all federated services like Facebook, Github etc that the user has used to
    /// authenticated himself and that he associated with this firebase auth account.
    pub providerUserInfo: Option<Vec<ProviderUserInfo>>,
    pub photoUrl: Option<String>,
    /// True if the account is disabled. A disabled account cannot login anymore.
    pub disabled: Option<bool>,
    /// Last login datetime in UTC
    pub lastLoginAt: Option<String>,
    /// Created datetime in UTC
    pub createdAt: Option<String>,
    /// True if email/password login have been used
    pub customAuth: Option<bool>,
}

/// Your user information query might return zero, one or more [`FirebaseAuthUser`] structures.
#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FirebaseAuthUserResponse {
    pub kind: String,
    pub users: Vec<FirebaseAuthUser>,
}

#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize, Serialize)]
struct UserRequest {
    pub idToken: String,
}

impl UserRequest {
    fn new(id_token: String) -> UserRequest {
        UserRequest { idToken: id_token }
    }
}

#[inline]
fn firebase_auth_url(v: &str, v2: &str) -> String {
    format!(
        "https://www.googleapis.com/identitytoolkit/v3/relyingparty/{}?key={}",
        v, v2
    )
}

#[inline]
fn user_info_internal(
    auth: String,
    api_key: &str,
    firebase_user_id: &str,
) -> Result<FirebaseAuthUserResponse> {
    let url = firebase_auth_url("getAccountInfo", api_key);

    let mut resp = Client::new()
        .post(&url)
        .json(&UserRequest::new(auth))
        .send()?;

    extract_google_api_error(&mut resp, || firebase_user_id.to_owned())?;

    Ok(resp.json()?)
}

/// Retrieve information about the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub fn user_info(session: &user::Session) -> Result<FirebaseAuthUserResponse> {
    user_info_internal(session.bearer(), &session.api_key, &session.userid)
}

#[inline]
fn user_remove_internal(auth: String, api_key: &str, firebase_user_id: &str) -> Result<()> {
    let url = firebase_auth_url("deleteAccount", api_key);
    let mut resp = Client::new()
        .post(&url)
        .json(&UserRequest::new(auth))
        .send()?;

    extract_google_api_error(&mut resp, || firebase_user_id.to_owned())?;
    Ok({})
}

/// Removes the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub fn user_remove(session: &user::Session) -> Result<()> {
    user_remove_internal(session.bearer(), &session.api_key, &session.userid)
}
