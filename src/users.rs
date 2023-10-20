//! # Firebase Auth API - User information
//!
//! Retrieve firebase user information

use super::errors::{extract_google_api_error, Result};

use super::sessions::{service_account, user};
use serde::{Deserialize, Serialize};

use crate::errors::extract_google_api_error_async;
use crate::{FirebaseAuthBearer, FirebaseAuthBearerAsync};

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
#[derive(Debug, Default, Deserialize, Serialize)]
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
    format!("https://identitytoolkit.googleapis.com/v1/accounts:{}?key={}", v, v2)
}

/// Retrieve information about the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub fn user_info(session: &user::BlockingSession) -> Result<FirebaseAuthUserResponse> {
    let url = firebase_auth_url("lookup", &session.api_key);

    let resp = session
        .client()
        .post(url)
        .json(&UserRequest {
            idToken: session.access_token(),
        })
        .send()?;

    let resp = extract_google_api_error(resp, || session.user_id.to_owned())?;

    Ok(resp.json()?)
}

/// Removes the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub fn user_remove(session: &user::BlockingSession) -> Result<()> {
    let url = firebase_auth_url("delete", &session.api_key);
    let resp = session
        .client()
        .post(url)
        .json(&UserRequest {
            idToken: session.access_token(),
        })
        .send()?;

    extract_google_api_error(resp, || session.user_id.to_owned())?;
    Ok(())
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

fn sign_up_in(
    session: &service_account::BlockingSession,
    email: &str,
    password: &str,
    action: &str,
) -> Result<user::BlockingSession> {
    let url = firebase_auth_url(action, &session.credentials.api_key);
    let resp = session
        .client()
        .post(url)
        .json(&SignInUpUserRequest {
            email: email.to_owned(),
            password: password.to_owned(),
            returnSecureToken: true,
        })
        .send()?;

    let resp = extract_google_api_error(resp, || email.to_owned())?;

    let resp: SignInUpUserResponse = resp.json()?;

    user::BlockingSession::new(
        &session.credentials,
        Some(&resp.localId),
        Some(&resp.idToken),
        Some(&resp.refreshToken),
    )
}

/// Creates the firebase auth user with the given email and password and returns
/// a user session.
///
/// Error codes:
/// EMAIL_EXISTS: The email address is already in use by another account.
/// OPERATION_NOT_ALLOWED: Password sign-in is disabled for this project.
/// TOO_MANY_ATTEMPTS_TRY_LATER: We have blocked all requests from this device due to unusual activity. Try again later.
pub fn sign_up(
    session: &service_account::BlockingSession,
    email: &str,
    password: &str,
) -> Result<user::BlockingSession> {
    sign_up_in(session, email, password, "signUp")
}

/// Signs in with the given email and password and returns a user session.
///
/// Error codes:
/// EMAIL_NOT_FOUND: There is no user record corresponding to this identifier. The user may have been deleted.
/// INVALID_PASSWORD: The password is invalid or the user does not have a password.
/// USER_DISABLED: The user account has been disabled by an administrator.
pub fn sign_in(
    session: &service_account::BlockingSession,
    email: &str,
    password: &str,
) -> Result<user::BlockingSession> {
    sign_up_in(session, email, password, "signInWithPassword")
}

/// ASYNC

/// Retrieve information about the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub async fn async_user_info(session: &mut user::AsyncSession) -> Result<FirebaseAuthUserResponse> {
    let url = firebase_auth_url("lookup", &session.api_key);

    let resp = session
        .client_async()
        .post(&url)
        .json(&UserRequest {
            idToken: session.access_token().await.to_string(),
        })
        .send()
        .await?;

    let resp = extract_google_api_error_async(resp, || session.user_id.to_owned()).await?;

    Ok(resp.json().await?)
}

/// Retrieve information about the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
async fn async_update_user(
    session: &mut user::AsyncSession,
    email: Option<&str>,
    password: Option<&str>,
) -> Result<Option<UpdateUser>> {
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:update?key={}",
        session.api_key,
    );

    let resp = session
        .client_async()
        .post(url)
        .header("Content-Type", "application/json")
        .json(&UpdateUserPayload {
            id_token: &session.access_token().await,
            email,
            password,
            return_secure_token: false,
        })
        .send()
        .await?;
    if resp.status() != 200 {
        extract_google_api_error_async(resp, || session.user_id.to_owned()).await?;
        return Ok(None);
    } else {
        let body = resp.json::<UpdateUser>().await?;
        return Ok(Some(body));
    }
}

// Change Email/Password
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserPayload<'a> {
    id_token: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,

    return_secure_token: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUser {
    pub kind: String,
    pub local_id: String,
    pub email: String,
    pub provider_user_info: Vec<ProviderUserInfo>,
    pub password_hash: String,
    pub email_verified: bool,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<String>,
}

async fn async_send_oob_code(
    session: &mut user::AsyncSession,
    request_type: &str,
    email: Option<&str>,
) -> Result<Option<SendOobCode>> {
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:sendOobCode?key={}",
        session.api_key,
    );

    let resp = session
        .client_async()
        .post(url)
        .header("Content-Type", "application/json")
        .json(&SendOobCodePayload {
            request_type,
            id_token: &session.access_token().await,
            email,
        })
        .send()
        .await?;
    if resp.status() != 200 {
        extract_google_api_error_async(resp, || session.user_id.to_owned()).await?;
        return Ok(None);
    } else {
        let body = resp.json::<SendOobCode>().await?;
        return Ok(Some(body));
    }
}

// Email Verification
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SendOobCodePayload<'a> {
    request_type: &'a str,
    id_token: &'a str,
    email: Option<&'a str>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendOobCode {
    pub kind: String,
    pub email: String,
}

/// Update a user's email
pub async fn change_email(session: &mut user::AsyncSession, email: &str) -> Result<Option<UpdateUser>> {
    async_update_user(session, Some(email), None).await
}

/// Update a user's password
pub async fn change_password(session: &mut user::AsyncSession, password: &str) -> Result<Option<UpdateUser>> {
    async_update_user(session, None, Some(password)).await
}

/// Send password reset email
pub async fn reset_password(session: &mut user::AsyncSession, email: &str) -> Result<Option<SendOobCode>> {
    async_send_oob_code(session, "PASSWORD_RESET", Some(email)).await
}

/// Send email verification message
pub async fn verify_email(session: &mut user::AsyncSession) -> Result<Option<SendOobCode>> {
    async_send_oob_code(session, "VERIFY_EMAIL", None).await
}

/// Removes the firebase auth user associated with the given user session
///
/// Error codes:
/// - INVALID_ID_TOKEN
/// - USER_NOT_FOUND
pub async fn async_user_remove(session: &mut user::AsyncSession) -> Result<()> {
    let url = firebase_auth_url("delete", &session.api_key);
    let resp = session
        .client_async()
        .post(&url)
        .json(&UserRequest {
            idToken: session.access_token().await.to_string(),
        })
        .send()
        .await?;

    extract_google_api_error_async(resp, || session.user_id.to_owned()).await?;
    Ok(())
}

async fn async_sign_up_in(
    session: &service_account::AsyncSession,
    email: &str,
    password: &str,
    action: &str,
) -> Result<user::AsyncSession> {
    let url = firebase_auth_url(action, &session.credentials.api_key);
    let resp = session
        .client_async()
        .post(&url)
        .json(&SignInUpUserRequest {
            email: email.to_owned(),
            password: password.to_owned(),
            returnSecureToken: true,
        })
        .send()
        .await?;

    let resp = extract_google_api_error_async(resp, || email.to_owned()).await?;

    let resp: SignInUpUserResponse = resp.json().await?;

    user::AsyncSession::new(
        &session.credentials,
        Some(&resp.localId),
        Some(&resp.idToken),
        Some(&resp.refreshToken),
    )
    .await
}

/// Creates the firebase auth user with the given email and password and returns
/// a user session.
///
/// Error codes:
/// EMAIL_EXISTS: The email address is already in use by another account.
/// OPERATION_NOT_ALLOWED: Password sign-in is disabled for this project.
/// TOO_MANY_ATTEMPTS_TRY_LATER: We have blocked all requests from this device due to unusual activity. Try again later.
pub async fn async_sign_up(
    session: &service_account::AsyncSession,
    email: &str,
    password: &str,
) -> Result<user::AsyncSession> {
    async_sign_up_in(session, email, password, "signUp").await
}

/// Signs in with the given email and password and returns a user session.
///
/// Error codes:
/// EMAIL_NOT_FOUND: There is no user record corresponding to this identifier. The user may have been deleted.
/// INVALID_PASSWORD: The password is invalid or the user does not have a password.
/// USER_DISABLED: The user account has been disabled by an administrator.
pub async fn async_sign_in(
    session: &service_account::AsyncSession,
    email: &str,
    password: &str,
) -> Result<user::AsyncSession> {
    async_sign_up_in(session, email, password, "signInWithPassword").await
}
