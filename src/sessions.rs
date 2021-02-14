//! # Authentication Session - Contains non-persistent access tokens
//!
//! A session can be either for a service-account or impersonated via a firebase auth user id.

use super::credentials;
use super::errors::{extract_google_api_error_async, FirebaseError};
use super::jwt::{create_jwt, verify_access_token, AuthClaimsJWT, JWT_AUDIENCE_FIRESTORE, JWT_AUDIENCE_IDENTITY};
use super::FirebaseAuthBearer;

use std::time::Duration;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ops::Deref;
use std::slice::Iter;

pub mod user {
    use super::*;
    use crate::jwt;
    use credentials::Credentials;

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
    /// Firestore rules will restrict your access.
    pub struct Session {
        /// The firebase auth user id
        pub user_id: String,
        /// The refresh token, if any. Such a token allows you to generate new, valid access tokens.
        /// This library will handle this for you, if for example your current access token expired.
        pub refresh_token: Option<String>,
        /// The firebase projects API key, as defined in the credentials object
        pub api_key: String,
        access_token_: RefCell<String>,
        project_id_: String,
        /// Returns a tokio runtime for the blocking API feature
        #[cfg(feature = "blocking")]
        pub rt: tokio::runtime::Runtime,
        /// The http client for async operations. Replace or modify the client if you have special demands like proxy support
        pub client_async: reqwest::Client,
    }

    impl super::FirebaseAuthBearer for Session {
        fn project_id(&self) -> &str {
            &self.project_id_
        }

        fn access_token(&self) -> String {
            self.access_token_.borrow().clone()
        }

        #[cfg(feature = "blocking")]
        fn rt(&self) -> &tokio::runtime::Runtime {
            &self.rt
        }

        fn client_async(&self) -> &reqwest::Client {
            &self.client_async
        }
    }

    /// Gets a new access token via an api_key and a refresh_token.
    /// This is a blocking operation.
    async fn get_new_access_token(
        client: &reqwest::Client,
        api_key: &str,
        refresh_token: &str,
    ) -> Result<RefreshTokenToAccessTokenResponse, FirebaseError> {
        let request_body = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token)];

        let url = refresh_to_access_endpoint(api_key);
        let response = client.post(&url).form(&request_body).send().await?;
        Ok(response.json().await?)
    }

    #[allow(non_snake_case)]
    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct CustomJwtToFirebaseID {
        token: String,
        returnSecureToken: bool,
    }

    impl CustomJwtToFirebaseID {
        fn new(token: String, with_refresh_token: bool) -> Self {
            CustomJwtToFirebaseID {
                token,
                returnSecureToken: with_refresh_token,
            }
        }
    }

    #[allow(non_snake_case)]
    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct CustomJwtToFirebaseIDResponse {
        kind: Option<String>,
        idToken: String,
        refreshToken: Option<String>,
        expiresIn: Option<String>,
    }

    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct RefreshTokenToAccessTokenResponse {
        /// The remaining lifetime of the access token in seconds.
        expires_in: i64,
        /// The type of token returned. At this time, this field's value is always set to Bearer.
        token_type: String,
        /// A token that you can use to obtain a new access token.
        /// Refresh tokens are valid until the user revokes access.
        refresh_token: String,
        /// Note: This property is only returned if your request included an identity scope, such as openid, profile, or email.
        /// The value is a JSON Web Token (JWT) that contains digitally signed identity information about the user.
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
        /// Async support: This is a blocking operation.
        ///
        /// See:
        /// * https://firebase.google.com/docs/reference/rest/auth#section-refresh-token
        /// * https://firebase.google.com/docs/auth/admin/create-custom-tokens#create_custom_tokens_using_a_third-party_jwt_library
        pub async fn new(
            credentials: &Credentials,
            user_id: Option<&str>,
            firebase_tokenid: Option<&str>,
            refresh_token: Option<&str>,
        ) -> Result<Session, FirebaseError> {
            // Check if current tokenid is still valid
            if let Some(firebase_tokenid) = firebase_tokenid {
                let r = Session::by_access_token(credentials, firebase_tokenid);
                if r.is_ok() {
                    let mut r = r.unwrap();
                    r.refresh_token = refresh_token.and_then(|f| Some(f.to_owned()));
                    return Ok(r);
                }
            }

            // Check if refresh_token is already sufficient
            if let Some(refresh_token) = refresh_token {
                let r = Session::by_refresh_token(credentials, refresh_token).await;
                if r.is_ok() {
                    return r;
                }
            }

            // Neither refresh token nor access token worked or are provided.
            // Try to get new new tokens for the given user_id via the REST API and the service-account credentials.
            if let Some(user_id) = user_id {
                let r = Session::by_user_id(credentials, user_id, true).await;
                if r.is_ok() {
                    return r;
                }
            }

            Err(FirebaseError::Generic("No parameter given"))
        }

        /// Create a new firestore user session via a valid refresh_token
        ///
        /// Arguments:
        /// - `credentials` The credentials
        /// - `refresh_token` A refresh token.
        ///
        /// Async support: This is a blocking operation.
        pub async fn by_refresh_token(credentials: &Credentials, refresh_token: &str) -> Result<Session, FirebaseError> {
            let client = reqwest::Client::new();
            let r: RefreshTokenToAccessTokenResponse = get_new_access_token(&client, &credentials.api_key, refresh_token).await?;
            Ok(Session {
                user_id: r.user_id,
                access_token_: RefCell::new(r.id_token),
                refresh_token: Some(r.refresh_token),
                project_id_: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
                #[cfg(feature = "blocking")]
                rt: tokio::runtime::Builder::new_current_thread().enable_all().build()
                    .map_err(|_| FirebaseError::Generic("Failed to create tokio runtime"))?,
                client_async: client,
            })
        }

        /// Create a new firestore user session with a fresh access token.
        ///
        /// Arguments:
        /// - `credentials` The credentials
        /// - `user_id` The firebase Authentication user id. Usually a string of about 30 characters like "Io2cPph06rUWM3ABcIHguR3CIw6v1".
        /// - `with_refresh_token` A refresh token is returned as well. This should be persisted somewhere for later reuse.
        ///    Google generates only a few dozens of refresh tokens before it starts to invalidate already generated ones.
        ///    For short lived, immutable, non-persisting services you do not want a refresh token.
        ///
        pub async fn by_user_id(
            credentials: &Credentials,
            user_id: &str,
            with_refresh_token: bool,
        ) -> Result<Session, FirebaseError> {
            let scope: Option<Iter<String>> = None;
            let jwt = create_jwt(
                &credentials,
                scope,
                Duration::from_secs(60 * 60),
                None,
                Some(user_id.to_owned()),
                JWT_AUDIENCE_IDENTITY,
            )?;
            let secret = credentials
                .keys
                .secret
                .as_ref()
                .ok_or(FirebaseError::Generic("No private key added via add_keypair_key!"))?;
            let encoded = jwt.encode(&secret.deref())?.encoded()?.encode();

            let client = reqwest::Client::new();
            let resp = client
                .post(&token_endpoint(&credentials.api_key))
                .json(&CustomJwtToFirebaseID::new(encoded, with_refresh_token))
                .send().await?;
            let resp = extract_google_api_error_async(resp, || user_id.to_owned()).await?;
            let r: CustomJwtToFirebaseIDResponse = resp.json().await?;

            Ok(Session {
                user_id: user_id.to_owned(),
                access_token_: RefCell::new(r.idToken),
                refresh_token: r.refreshToken,
                project_id_: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
                #[cfg(feature = "blocking")]
                rt: tokio::runtime::Builder::new_current_thread().enable_all().build()
                    .map_err(|_| FirebaseError::Generic("Failed to create tokio runtime"))?,
                client_async: client,
            })
        }

        pub fn by_access_token(credentials: &Credentials, firebase_tokenid: &str) -> Result<Session, FirebaseError> {
            let result = verify_access_token(&credentials, firebase_tokenid)?;
            Ok(Session {
                user_id: result.subject,
                project_id_: result.audience,
                access_token_: RefCell::new(firebase_tokenid.to_owned()),
                refresh_token: None,
                api_key: credentials.api_key.clone(),
                #[cfg(feature = "blocking")]
                rt: tokio::runtime::Builder::new_current_thread().enable_all().build()
                    .map_err(|_| FirebaseError::Generic("Failed to create tokio runtime"))?,
                client_async: reqwest::Client::new(),
            })
        }

        /// Requests a new access token from the OAuth server, if the current token has expired.
        ///
        /// You should call this method periodically. The method returns the expire time in seconds.
        pub async fn check_refresh_access_token(self: &mut Self) -> Result<i64, FirebaseError> {
            let jwt = self.access_token_.borrow();
            let jwt = jwt.as_str();

            let exp_in_sec = jwt::expires(&jwt)?.as_secs() as i64;
            if exp_in_sec > 0 {
                return Ok(exp_in_sec);
            }

            get_new_access_token(&self.client_async, &self.api_key, jwt).await
                .map(|response| {
                    self.access_token_.swap(&RefCell::new(response.id_token.clone()));
                    return response.expires_in;
                }
                )
        }
    }
}

/// Find the service account session defined in here
pub mod service_account {
    use super::*;
    use credentials::Credentials;
    use crate::jwt;

    use std::cell::RefCell;
    use std::ops::Deref;

    /// Service account session
    pub struct Session {
        /// The google credentials
        pub credentials: Credentials,
        /// Returns a tokio runtime for the blocking API feature
        #[cfg(feature = "blocking")]
        pub rt: tokio::runtime::Runtime,
        /// The http client for async operations. Replace or modify the client if you have special demands like proxy support
        pub client_async: reqwest::Client,
        jwt: RefCell<AuthClaimsJWT>,
        access_token_: RefCell<String>,
    }

    impl super::FirebaseAuthBearer for Session {
        fn project_id(&self) -> &str {
            &self.credentials.project_id
        }

        fn access_token(&self) -> String {
            self.access_token_.borrow().clone()
        }

        #[cfg(feature = "blocking")]
        fn rt(&self) -> &tokio::runtime::Runtime {
            &self.rt
        }

        fn client_async(&self) -> &reqwest::Client {
            &self.client_async
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
                std::time::Duration::from_secs(60 * 60),
                None,
                None,
                JWT_AUDIENCE_FIRESTORE,
            )?;
            let secret = credentials
                .keys
                .secret
                .as_ref()
                .ok_or(FirebaseError::Generic("No private key added via add_keypair_key!"))?;
            let encoded = jwt.encode(&secret.deref())?.encoded()?.encode();

            Ok(Session {
                access_token_: RefCell::new(encoded),
                jwt: RefCell::new(jwt),
                credentials,
                #[cfg(feature = "blocking")]
                rt: tokio::runtime::Builder::new_current_thread().enable_all().build()
                    .map_err(|_| FirebaseError::Generic("Failed to create tokio runtime"))?,
                client_async: reqwest::Client::new(),
            })
        }

        /// Checks the access token expiry and refreshes if necessary.
        ///
        /// Implementation details: If the jwt, used as authentication bearer token,
        /// issue_at field is older than 5 minutes, it will be updated to the current time.
        pub async fn check_refresh_access_token(self: &mut Self) -> Result<std::time::Duration, FirebaseError> {
            let mut jwt = self.jwt.borrow_mut();

            let exp = jwt::expires_jwt(&jwt)?;
            if exp.as_secs() > 60 * 5 {
                return Ok(exp);
            }

            match self.credentials.keys.secret.as_ref() {
                None => Err(FirebaseError::Generic("No credentials key, cannot sign access tokens!")),
                Some(secret) => {
                    let encoded_jwt = jwt::jwt_update_expiry_and_sign(&mut jwt, secret, std::time::Duration::from_secs(60 * 60))?;
                    self.access_token_.swap(&RefCell::new(encoded_jwt));
                    Ok(exp)
                }
            }
        }
    }
}
