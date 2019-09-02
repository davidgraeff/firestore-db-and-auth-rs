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

#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProviderUserInfo {
    pub providerId: String,
    pub federatedId: String,
    pub displayName: Option<String>,
    pub photoUrl: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FirebaseAuthUser {
    pub localId: Option<String>,
    pub email: Option<String>,
    pub emailVerified: Option<bool>,
    pub displayName: Option<String>,
    pub providerUserInfo: Option<Vec<ProviderUserInfo>>,
    pub photoUrl: Option<String>,
    pub disabled: Option<bool>,
    pub lastLoginAt: Option<String>,
    pub createdAt: Option<String>,
    pub customAuth: Option<bool>,
}

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

pub fn userremove(auth: &Session) -> Result<()> {
    let url = format!(firebase_auth_url!(), "deleteAccount", auth.api_key);
    Client::new()
        .post(&url)
        .bearer_auth(auth.bearer.to_owned())
        .send()?;

    Ok({})
}
