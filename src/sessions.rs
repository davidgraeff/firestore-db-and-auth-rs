//! # Authentication Session - Contains non-persistent access tokens
//!
//! A session can be either for a service-account or impersonated via a firebase auth user id.

use super::credentials;
use super::errors::{extract_google_api_error, FirebaseError};
use super::jwt::{
    create_jwt, jwt_update_expiry_if, verify_access_token, AuthClaimsJWT, JWT_AUDIENCE_FIRESTORE,
    JWT_AUDIENCE_IDENTITY,
};
use super::FirebaseAuthBearer;
use std::slice::Iter;

pub mod user {
    use super::*;
    use credentials::Credentials;

    use chrono::Duration;
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use std::ops::Deref;

    #[inline]
    fn token_endpoint(v: &str) -> String {
        format!(
            "https://www.googleapis.com/identitytoolkit/v3/relyingparty/verifyCustomToken?key={}",
            v
        )
    }

    #[inline]
    fn refresh_to_access_endpoint(v: &str) -> String {
        format!("https://securetoken.googleapis.com/v1/token?key={}", v)
    }

    /// An impersonated session.
    /// If you access FireStore with such a session, FireStore rules might restrict access to data.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Session {
        pub userid: String,
        pub refresh_token: Option<String>,
        pub api_key: String,
        pub bearer: String,
        pub projectid: String,
    }

    impl<'a> super::FirebaseAuthBearer<'a> for Session {
        fn projectid(&'a self) -> &'a str {
            &self.projectid
        }
        fn bearer(&'a self) -> String {
            self.bearer.clone()
        }
    }

    #[allow(non_snake_case)]
    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct CustomJwtToFirebaseID {
        token: String,
        returnSecureToken: bool,
    }

    impl CustomJwtToFirebaseID {
        fn new(token: String) -> Self {
            CustomJwtToFirebaseID {
                token,
                returnSecureToken: true,
            }
        }
    }

    #[allow(non_snake_case)]
    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct CustomJwtToFirebaseIDResponse {
        kind: Option<String>,
        idToken: String,
        refreshToken: String,
        expiresIn: String,
    }

    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct RefreshTokenToAccessTokenResponse {
        expires_in: String,
        token_type: String,
        refresh_token: String,
        id_token: String,
        user_id: String,
        project_id: String,
    }

    impl Session {
        /// Create an impersonated session
        ///
        /// If the optionally provided access token is still valid, it will be used.
        /// If the access token is not valid anymore, but the given refresh token is, it will be used to retrieve a new access token.
        ///
        /// If neither refresh token nor access token work are provided or valid, the service account credentials will be used to generate
        /// a new impersonated refresh and access token for the given user.
        ///
        /// If none of the parameters are given, the function will error out.
        ///
        /// See:
        /// * https://firebase.google.com/docs/reference/rest/auth#section-refresh-token
        /// * https://firebase.google.com/docs/auth/admin/create-custom-tokens#create_custom_tokens_using_a_third-party_jwt_library
        pub fn new(
            credentials: &Credentials,
            user_id: Option<&str>,
            firebase_tokenid: Option<&str>,
            refresh_token: Option<&str>,
        ) -> Result<Session, FirebaseError> {
            // Check if current tokenid is still valid
            if let Some(firebase_tokenid) = firebase_tokenid {
                let r = Session::by_access_token(credentials, firebase_tokenid);
                if r.is_ok() {
                    return r;
                }
            }

            // Check if refresh_token is already sufficient
            if let Some(refresh_token) = refresh_token {
                let r = Session::by_refresh_token(credentials, refresh_token);
                if r.is_ok() {
                    return r;
                }
            }

            // Check if refresh_token is already sufficient
            if let Some(user_id) = user_id {
                let r = Session::by_user_id(credentials, user_id);
                if r.is_ok() {
                    return r;
                }
            }

            Err(FirebaseError::Generic("No parameter given"))
        }

        /// Create a new firestore user session via a valid refresh_token
        pub fn by_refresh_token(
            credentials: &Credentials,
            refresh_token: &str,
        ) -> Result<Session, FirebaseError> {
            let request_body = vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ];

            let url = refresh_to_access_endpoint(&credentials.api_key);
            let client = Client::new();
            let ref mut response = client.post(&url).form(&request_body).send()?;
            let r: RefreshTokenToAccessTokenResponse = response.json()?;
            Ok(Session {
                userid: r.user_id,
                bearer: r.id_token,
                refresh_token: Some(r.refresh_token),
                projectid: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
            })
        }

        /// Create a new firestore user session with a fresh access token and new refresh token
        ///
        /// If possible, use existing tokens and [`new`] instead.
        pub fn by_user_id(
            credentials: &Credentials,
            user_id: &str,
        ) -> Result<Session, FirebaseError> {
            let scope: Option<Iter<String>> = None;
            let jwt = create_jwt(
                &credentials,
                scope,
                Duration::hours(1),
                None,
                Some(user_id.to_owned()),
                JWT_AUDIENCE_IDENTITY,
            )?;
            let secret = credentials
                .keys
                .secret
                .as_ref()
                .ok_or(FirebaseError::Generic(
                    "No private key added via add_keypair_key!",
                ))?;
            let encoded = jwt.encode(&secret.deref())?.encoded()?.encode();

            let mut r = Client::new()
                .post(&token_endpoint(&credentials.api_key))
                .json(&CustomJwtToFirebaseID::new(encoded))
                .send()?;
            extract_google_api_error(&mut r, || user_id.to_owned())?;
            let r: CustomJwtToFirebaseIDResponse = r.json()?;

            Ok(Session {
                userid: user_id.to_owned(),
                bearer: r.idToken,
                refresh_token: Some(r.refreshToken),
                projectid: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
            })
        }

        pub fn by_access_token(
            credentials: &Credentials,
            firebase_tokenid: &str,
        ) -> Result<Session, FirebaseError> {
            let result = verify_access_token(&credentials, firebase_tokenid)?
                .ok_or(FirebaseError::Generic("Validation failed"))?;
            return Ok(Session {
                userid: result.subject,
                projectid: result.audience,
                bearer: firebase_tokenid.to_owned(),
                refresh_token: None,
                api_key: credentials.api_key.clone(),
            });
        }
    }
}

/// Find the service account session defined in here
pub mod service_account {
    use super::*;
    use credentials::Credentials;

    use chrono::Duration;
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use std::ops::Deref;

    /// Service account session
    #[derive(Serialize, Deserialize)]
    pub struct Session {
        pub credentials: Credentials,
        jwt: RefCell<AuthClaimsJWT>,
        bearer_cache: RefCell<String>,
    }

    impl<'a> super::FirebaseAuthBearer<'a> for Session {
        fn projectid(&'a self) -> &'a str {
            &self.credentials.project_id
        }
        /// Return the encoded jwt to be used as bearer token. If the jwt
        /// issue_at is older than 50 minutes, it will be updated to the current time.
        fn bearer(&'a self) -> String {
            let mut jwt = self.jwt.borrow_mut();

            if jwt_update_expiry_if(&mut jwt, 50) {
                if let Some(secret) = self.credentials.keys.secret.as_ref() {
                    if let Ok(v) = self.jwt.borrow().encode(&secret.deref()) {
                        if let Ok(v2) = v.encoded() {
                            self.bearer_cache.swap(&RefCell::new(v2.encode()));
                        }
                    }
                }
            }

            self.bearer_cache.borrow().clone()
        }
    }

    impl Session {
        /// You need a service account credentials file, provided by the Google Cloud console.
        ///
        /// The service account session can be used to interact with the FireStore API as well as
        /// FireBase Auth.
        ///
        /// A custom jwt is created and signed with the service account private key. This jwt is used
        /// as bearer token.
        ///
        /// See https://developers.google.com/identity/protocols/OAuth2ServiceAccount
        pub fn new(credentials: Credentials) -> Result<Session, FirebaseError> {
            let scope: Option<Iter<String>> = None;
            let jwt = create_jwt(
                &credentials,
                scope,
                Duration::hours(1),
                None,
                None,
                JWT_AUDIENCE_FIRESTORE,
            )?;
            let secret = credentials
                .keys
                .secret
                .as_ref()
                .ok_or(FirebaseError::Generic(
                    "No private key added via add_keypair_key!",
                ))?;
            let encoded = jwt.encode(&secret.deref())?.encoded()?.encode();

            Ok(Session {
                bearer_cache: RefCell::new(encoded),
                jwt: RefCell::new(jwt),
                credentials,
            })
        }
    }
}
