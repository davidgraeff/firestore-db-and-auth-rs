//! # Firebase Auth API - User information
//!
//! Retrieve firebase user information

use super::errors::{extract_google_api_error, Result};

use super::sessions::{user, service_account};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::FirebaseAuthBearer;
use reqwest::Client;

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
#[derive(Debug, Default, Deserialize)]
pub struct FirebaseAuthUserResponse {
    pub kind: String,
    pub users: Vec<FirebaseAuthUser>,
}

#[allow(non_snake_case)]
#[derive(Debug, Default, Serialize)]
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
        "https://identitytoolkit.googleapis.com/v1/accounts:{}?key={}",
        v, v2
    )
}

#[inline]
fn user_info_internal(
    auth: String,
    api_key: &str,
    firebase_user_id: &str,
) -> Result<FirebaseAuthUserResponse> {
    let url = firebase_auth_url("lookup", api_key);

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
    user_info_internal(session.access_token(), &session.api_key, &session.user_id)
}

#[inline]
fn user_remove_internal(auth: String, api_key: &str, firebase_user_id: &str) -> Result<()> {
    let url = firebase_auth_url("delete", api_key);
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
    user_remove_internal(session.access_token(), &session.api_key, &session.user_id)
}


#[allow(non_snake_case)]
#[derive(Default, Deserialize)]
struct CreateUserResponse {
    pub localId: String,
    pub idToken: String,
    // access token
    pub refreshToken: String,
    pub email: String,
}

/// Creates the firebase auth user with the given email and password and returns
/// a user session.
///
/// Error codes:
/// EMAIL_EXISTS: The email address is already in use by another account.
/// OPERATION_NOT_ALLOWED: Password sign-in is disabled for this project.
/// TOO_MANY_ATTEMPTS_TRY_LATER: We have blocked all requests from this device due to unusual activity. Try again later.
pub fn create_user(session: &service_account::Session, email: &str, password: &str) -> Result<user::Session> {
    let url = firebase_auth_url("signUp", &session.credentials.api_key);
    let mut resp = Client::new()
        .post(&url)
        .json(&json!({
            "email": email,
            "password": password,
            "returnSecureToken": true,
        }))
        .send()?;

    extract_google_api_error(&mut resp, || email.to_owned())?;

    let resp: CreateUserResponse = resp.json()?;

    Ok(user::Session::new(&session.credentials, Some(&resp.localId), Some(&resp.idToken), Some(&resp.refreshToken))?)
}
