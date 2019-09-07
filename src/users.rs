//! # Firebase Auth API - User information
//!
//! Retrieve firebase user information

use super::errors::{extract_google_api_error, Result};

use super::sessions::{user, service_account};
use serde::{Deserialize, Serialize};

use crate::FirebaseAuthBearer;

/// A federated services like Facebook, Github etc that the user has used to
/// authenticated himself and that he associated with this firebase auth account.
#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize)]
pub struct ProviderUserInfo {
    pub providerId: String,
    pub federatedId: String,
    pub displayName: Option<String>,
    pub photoUrl: Option<String>,
}

/// Users id, email, display name and a few more information
#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize)]
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
#[derive(Serialize)]
struct UserRequest {
    pub idToken: String,
}

#[inline]
fn firebase_auth_url(v: &str, v2: &str) -> String {
    format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:{}?key={}",
        v, v2
    )
}

/// Retrieve information about the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub fn user_info(session: &user::Session) -> Result<FirebaseAuthUserResponse> {
    let url = firebase_auth_url("lookup", &session.api_key);

    let mut resp = session.client()
        .post(&url)
        .json(&UserRequest { idToken: session.access_token() })
        .send()?;

    extract_google_api_error(&mut resp, || session.user_id.to_owned())?;

    Ok(resp.json()?)
}

/// Removes the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub fn user_remove(session: &user::Session) -> Result<()> {
    let url = firebase_auth_url("delete", &session.api_key);
    let mut resp = session.client()
        .post(&url)
        .json(&UserRequest { idToken: session.access_token() })
        .send()?;

    extract_google_api_error(&mut resp, || session.user_id.to_owned())?;
    Ok({})
}


#[allow(non_snake_case)]
#[derive(Default, Deserialize)]
struct SignInUpUserResponse {
    localId: String,
    idToken: String,
    refreshToken: String,
}

#[allow(non_snake_case)]
#[derive(Serialize)]
struct SignInUpUserRequest {
    pub email: String,
    pub password: String,
    pub returnSecureToken: bool,
}

fn sign_up_in(session: &service_account::Session, email: &str, password: &str, action: &str) -> Result<user::Session> {
    let url = firebase_auth_url(action, &session.credentials.api_key);
    let mut resp = session.client()
        .post(&url)
        .json(&SignInUpUserRequest { email: email.to_owned(), password: password.to_owned(), returnSecureToken: true })
        .send()?;

    extract_google_api_error(&mut resp, || email.to_owned())?;

    let resp: SignInUpUserResponse = resp.json()?;

    Ok(user::Session::new(&session.credentials, Some(&resp.localId), Some(&resp.idToken), Some(&resp.refreshToken))?)
}


/// Creates the firebase auth user with the given email and password and returns
/// a user session.
///
/// Error codes:
/// EMAIL_EXISTS: The email address is already in use by another account.
/// OPERATION_NOT_ALLOWED: Password sign-in is disabled for this project.
/// TOO_MANY_ATTEMPTS_TRY_LATER: We have blocked all requests from this device due to unusual activity. Try again later.
pub fn sign_up(session: &service_account::Session, email: &str, password: &str) -> Result<user::Session> {
    sign_up_in(session, email, password, "signUp")
}

/// Signs in with the given email and password and returns a user session.
///
/// Error codes:
/// EMAIL_NOT_FOUND: There is no user record corresponding to this identifier. The user may have been deleted.
/// INVALID_PASSWORD: The password is invalid or the user does not have a password.
/// USER_DISABLED: The user account has been disabled by an administrator.
pub fn sign_in(session: &service_account::Session, email: &str, password: &str) -> Result<user::Session> {
    sign_up_in(session, email, password, "signInWithPassword")
}
