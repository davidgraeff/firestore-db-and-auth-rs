//! # Firebase Auth API - User information
//!
//! Retrieve firebase user information

macro_rules! firebase_auth_url {
    () => {
        "https://www.googleapis.com/identitytoolkit/v3/relyingparty/{}?key={}"
    };
}

use super::errors::{FirebaseError, Result};

use super::sessions::user::Session;
use serde::{Deserialize, Serialize};

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

/// Retrieve information about the firebase auth user associated with the given session
pub fn userinfo(auth: &Session) -> Result<FirebaseAuthUserResponse> {
    let url = format!(firebase_auth_url!(), "getAccountInfo", auth.api_key);

    let request = UserRequest {
        idToken: auth.bearer.to_owned(),
    };

    let mut resp = Client::new().post(&url).json(&request).send()?;

    if resp.status() != 200 {
        return Err(FirebaseError::UnexpectedResponse(
            "User info: ",
            resp.status(),
            resp.text()?,
            serde_json::to_string_pretty(&request)?,
        ));
    }

    Ok(resp.json()?)
}

/// Removes the firebase auth user associated with the given session
pub fn userremove(auth: &Session) -> Result<()> {
    let url = format!(firebase_auth_url!(), "deleteAccount", auth.api_key);
    Client::new()
        .post(&url)
        .bearer_auth(auth.bearer.to_owned())
        .send()?;

    Ok({})
}
