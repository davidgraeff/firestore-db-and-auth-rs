//! # Authentication Session - Contains non-persistent access tokens
//!
//! A session can be either for a service-account or impersonated via a firebase auth user id.

#![allow(unused_imports)]
use super::credentials;
use super::errors::{extract_google_api_error, extract_google_api_error_async, FirebaseError};
use super::jwt::{
    create_jwt, is_expired, jwt_update_expiry_if, verify_access_token, AuthClaimsJWT, JWT_AUDIENCE_FIRESTORE,
    JWT_AUDIENCE_IDENTITY,
};
use super::FirebaseAuthBearer;

use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod user {
    use super::*;
    use crate::dto::{OAuthResponse, SignInWithIdpRequest};
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

    /// Default OAuth2 Providers supported by Firebase.
    /// see: * https://firebase.google.com/docs/projects/provisioning/configure-oauth?hl=en#add-idp
    pub enum OAuth2Provider {
        Apple,
        AppleGameCenter,
        Facebook,
        GitHub,
        Google,
        GooglePlayGames,
        LinkedIn,
        Microsoft,
        Twitter,
        Yahoo,
    }

    fn get_provider(provider: OAuth2Provider) -> String {
        match provider {
            OAuth2Provider::Apple => "apple.com".to_string(),
            OAuth2Provider::AppleGameCenter => "gc.apple.com".to_string(),
            OAuth2Provider::Facebook => "facebook.com".to_string(),
            OAuth2Provider::GitHub => "github.com".to_string(),
            OAuth2Provider::Google => "google.com".to_string(),
            OAuth2Provider::GooglePlayGames => "playgames.google.com".to_string(),
            OAuth2Provider::LinkedIn => "linkedin.com".to_string(),
            OAuth2Provider::Microsoft => "microsoft.com".to_string(),
            OAuth2Provider::Twitter => "twitter.com".to_string(),
            OAuth2Provider::Yahoo => "yahoo.com".to_string(),
        }
    }

    /// An impersonated session.
    /// Firestore rules will restrict your access.
    #[derive(Clone)]
    pub struct Session {
        /// The firebase auth user id
        pub user_id: String,
        /// The refresh token, if any. Such a token allows you to generate new, valid access tokens.
        /// This library will handle this for you, if for example your current access token expired.
        pub refresh_token: Option<String>,
        /// The firebase projects API key, as defined in the credentials object
        pub api_key: String,

        access_token_: Arc<RwLock<String>>,

        project_id_: String,
        /// The http client for async operations. Replace or modify the client if you have special demands like proxy support
        pub client: reqwest::Client,
    }

    #[async_trait::async_trait]
    impl super::FirebaseAuthBearer for Session {
        fn project_id(&self) -> &str {
            &self.project_id_
        }

        async fn access_token_unchecked(&self) -> String {
            self.access_token_.read().await.clone()
        }

        /// Returns the current access token.
        /// This method will automatically refresh your access token, if it has expired.
        ///
        /// If the refresh failed, this will return an empty string.
        async fn access_token(&self) -> String {
            // Let's keep the access token locked for writes for the entirety of this function,
            // so we don't have multiple refreshes going on at the same time
            let mut jwt = self.access_token_.write().await;

            if is_expired(&jwt, 0).unwrap() {
                // Unwrap: the token is always valid at this point
                if let Ok(response) = get_new_access_token(&self.api_key, &jwt).await {
                    *jwt = response.id_token.clone();
                    return response.id_token;
                } else {
                    // Failed to refresh access token. Return an empty string
                    return String::new();
                }
            }

            jwt.clone()
        }

        fn client(&self) -> &reqwest::Client {
            &self.client
        }
    }

    /// Gets a new access token via an api_key and a refresh_token.
    async fn get_new_access_token(
        api_key: &str,
        refresh_token: &str,
    ) -> Result<RefreshTokenToAccessTokenResponse, FirebaseError> {
        let request_body = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token)];

        let url = refresh_to_access_endpoint(api_key);
        let client = reqwest::Client::new();
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
                let r = Session::by_access_token(credentials, firebase_tokenid).await;
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
        pub async fn by_refresh_token(
            credentials: &Credentials,
            refresh_token: &str,
        ) -> Result<Session, FirebaseError> {
            let r: RefreshTokenToAccessTokenResponse =
                get_new_access_token(&credentials.api_key, refresh_token).await?;
            Ok(Session {
                user_id: r.user_id,
                access_token_: Arc::new(RwLock::new(r.id_token)),
                refresh_token: Some(r.refresh_token),
                project_id_: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
                client: reqwest::Client::new(),
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
                Duration::hours(1),
                None,
                Some(user_id.to_owned()),
                JWT_AUDIENCE_IDENTITY,
            )?;
            let secret_lock = credentials.keys.read().await;
            let secret = secret_lock
                .secret
                .as_ref()
                .ok_or(FirebaseError::Generic("No private key added via add_keypair_key!"))?;
            let encoded = jwt.encode(&secret.deref())?.encoded()?.encode();

            let resp = reqwest::Client::new()
                .post(&token_endpoint(&credentials.api_key))
                .json(&CustomJwtToFirebaseID::new(encoded, with_refresh_token))
                .send()
                .await?;
            let resp = extract_google_api_error_async(resp, || user_id.to_owned()).await?;
            let r: CustomJwtToFirebaseIDResponse = resp.json().await?;

            Ok(Session {
                user_id: user_id.to_owned(),
                access_token_: Arc::new(RwLock::new(r.idToken)),
                refresh_token: r.refreshToken,
                project_id_: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
                client: reqwest::Client::new(),
            })
        }

        /// Create a new firestore user session by a valid access token
        ///
        /// Remember that such a session cannot renew itself. As soon as the access token expired,
        /// no further operations can be issued by this session.
        ///
        /// No network operation is performed, the access token is only checked for its validity.
        ///
        /// Arguments:
        /// - `credentials` The credentials
        /// - `access_token` An access token, sometimes called a firebase id token.
        ///
        pub async fn by_access_token(credentials: &Credentials, access_token: &str) -> Result<Session, FirebaseError> {
            let result = verify_access_token(&credentials, access_token).await?;
            Ok(Session {
                user_id: result.subject,
                project_id_: result.audience,
                access_token_: Arc::new(RwLock::new(access_token.to_owned())),
                refresh_token: None,
                api_key: credentials.api_key.clone(),
                client: reqwest::Client::new(),
            })
        }

        /// Creates a new user session with OAuth2 provider token.
        /// If user don't exist it's create new user in firestore
        ///
        /// Arguments:
        /// - `credentials` The credentials.
        /// - `access_token` access_token provided by OAuth2 provider.
        /// - `request_uri` The URI to which the provider redirects the user back same as from .
        /// - `provider` OAuth2Provider enum: Apple, AppleGameCenter, Facebook, GitHub, Google, GooglePlayGames, LinkedIn, Microsoft, Twitter, Yahoo.
        /// - `with_refresh_token` A refresh token is returned as well. This should be persisted somewhere for later reuse.
        ///    Google generates only a few dozens of refresh tokens before it starts to invalidate already generated ones.
        ///    For short lived, immutable, non-persisting services you do not want a refresh token.
        ///
        pub async fn by_oauth2(
            credentials: &Credentials,
            access_token: String,
            provider: OAuth2Provider,
            request_uri: String,
            with_refresh_token: bool,
        ) -> Result<Session, FirebaseError> {
            let uri = "https://identitytoolkit.googleapis.com/v1/accounts:signInWithIdp?key=".to_owned()
                + &credentials.api_key;

            let post_body = format!("access_token={}&providerId={}", access_token, get_provider(provider));
            let return_idp_credential = true;
            let return_secure_token = true;

            let json = &SignInWithIdpRequest {
                post_body,
                request_uri,
                return_idp_credential,
                return_secure_token,
            };

            let response = reqwest::Client::new().post(&uri).json(&json).send().await?;

            let oauth_response: OAuthResponse = response.json().await?;

            self::Session::by_user_id(&credentials, &oauth_response.local_id, with_refresh_token).await
        }
    }
}

pub mod session_cookie {
    use super::*;

    pub static GOOGLE_OAUTH2_URL: &str = "https://accounts.google.com/o/oauth2/token";

    /// See https://cloud.google.com/identity-platform/docs/reference/rest/v1/projects/createSessionCookie
    #[inline]
    fn identitytoolkit_url(project_id: &str) -> String {
        format!(
            "https://identitytoolkit.googleapis.com/v1/projects/{}:createSessionCookie",
            project_id
        )
    }

    /// See https://cloud.google.com/identity-platform/docs/reference/rest/v1/CreateSessionCookieResponse
    #[derive(Debug, Deserialize)]
    struct CreateSessionCookieResponseDTO {
        #[serde(rename = "sessionCookie")]
        session_cookie_jwk: String,
    }

    /// https://cloud.google.com/identity-platform/docs/reference/rest/v1/projects/createSessionCookie
    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
    struct SessionLoginDTO {
        /// Required. A valid Identity Platform ID token.
        #[serde(rename = "idToken")]
        id_token: String,
        /// The number of seconds until the session cookie expires. Specify a duration in seconds, between five minutes and fourteen days, inclusively.
        #[serde(rename = "validDuration")]
        valid_duration: u64,
        #[serde(rename = "tenantId")]
        #[serde(skip_serializing_if = "Option::is_none")]
        tenant_id: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct Oauth2ResponseDTO {
        access_token: String,
    }

    /// Firebase Auth provides server-side session cookie management for traditional websites that rely on session cookies.
    /// This solution has several advantages over client-side short-lived ID tokens,
    /// which may require a redirect mechanism each time to update the session cookie on expiration:
    ///
    /// * Improved security via JWT-based session tokens that can only be generated using authorized service accounts.
    /// * Stateless session cookies that come with all the benefit of using JWTs for authentication.
    ///   The session cookie has the same claims (including custom claims) as the ID token, making the same permissions checks enforceable on the session cookies.
    /// * Ability to create session cookies with custom expiration times ranging from 5 minutes to 2 weeks.
    /// * Flexibility to enforce cookie policies based on application requirements: domain, path, secure, httpOnly, etc.
    /// * Ability to revoke session cookies when token theft is suspected using the existing refresh token revocation API.
    /// * Ability to detect session revocation on major account changes.
    ///
    /// See https://firebase.google.com/docs/auth/admin/manage-cookies
    ///
    /// The generated session cookie is a JWT that includes the firebase user id in the "sub" (subject) field.
    ///
    /// Arguments:
    /// - `credentials` The credentials
    /// - `id_token` An access token, sometimes called a firebase id token.
    /// - `duration` The cookie duration
    ///
    pub async fn create(
        credentials: &credentials::Credentials,
        id_token: String,
        duration: chrono::Duration,
    ) -> Result<String, FirebaseError> {
        // Generate the assertion from the admin credentials
        let assertion = crate::jwt::session_cookie::create_jwt_encoded(credentials, duration).await?;

        // Request Google Oauth2 to retrieve the access token in order to create a session cookie
        let client = reqwest::blocking::Client::new();
        let response_oauth2: Oauth2ResponseDTO = client
            .post(GOOGLE_OAUTH2_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &assertion),
            ])
            .send()?
            .json()?;

        // Create a session cookie with the access token previously retrieved
        let response_session_cookie_json: CreateSessionCookieResponseDTO = client
            .post(&identitytoolkit_url(&credentials.project_id))
            .bearer_auth(&response_oauth2.access_token)
            .json(&SessionLoginDTO {
                id_token,
                valid_duration: duration.num_seconds() as u64,
                tenant_id: None,
            })
            .send()?
            .json()?;

        Ok(response_session_cookie_json.session_cookie_jwk)
    }
}

/// Find the service account session defined in here
pub mod service_account {
    use crate::jwt::TokenValidationResult;

    use super::*;
    use credentials::Credentials;

    use chrono::Duration;
    use std::cell::RefCell;
    use std::ops::Deref;

    /// Service account session
    #[derive(Clone, Debug)]
    pub struct Session {
        /// The google credentials
        pub credentials: Credentials,
        /// The http client for async operations. Replace or modify the client if you have special demands like proxy support
        pub client: reqwest::Client,
        jwt: Arc<RwLock<AuthClaimsJWT>>,
        access_token_: Arc<RwLock<String>>,
    }

    #[async_trait::async_trait]
    impl super::FirebaseAuthBearer for Session {
        fn project_id(&self) -> &str {
            &self.credentials.project_id
        }

        /// Return the encoded jwt to be used as bearer token. If the jwt
        /// issue_at is older than 50 minutes, it will be updated to the current time.
        async fn access_token(&self) -> String {
            // Keeping the JWT and the access token in write mode so this area is
            // a single-entrace critical section for refreshes sake
            let mut access_token = self.access_token_.write().await;
            let maybe_jwt = {
                let mut jwt = self.jwt.write().await;

                if jwt_update_expiry_if(&mut jwt, 50) {
                    self.credentials
                        .keys
                        .read()
                        .await
                        .secret
                        .as_ref()
                        .and_then(|secret| jwt.clone().encode(&secret.deref()).ok())
                } else {
                    None
                }
            };

            if let Some(v) = maybe_jwt {
                if let Ok(v) = v.encoded() {
                    *access_token = v.encode();
                }
            }

            access_token.clone()
        }

        async fn access_token_unchecked(&self) -> String {
            self.access_token_.read().await.clone()
        }

        fn client(&self) -> &reqwest::Client {
            &self.client
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
        pub async fn new(credentials: Credentials) -> Result<Session, FirebaseError> {
            let scope: Option<Iter<String>> = None;
            let jwt = create_jwt(
                &credentials,
                scope,
                Duration::hours(1),
                None,
                None,
                JWT_AUDIENCE_FIRESTORE,
            )?;
            let encoded = {
                let secret_lock = credentials.keys.read().await;
                let secret = secret_lock
                    .secret
                    .as_ref()
                    .ok_or(FirebaseError::Generic("No private key added via add_keypair_key!"))?;
                jwt.encode(&secret.deref())?.encoded()?.encode()
            };

            Ok(Session {
                access_token_: Arc::new(RwLock::new(encoded)),
                jwt: Arc::new(RwLock::new(jwt)),

                credentials,
                client: reqwest::Client::new(),
            })
        }

        pub async fn verify_token(&self, token: &str) -> Result<TokenValidationResult, FirebaseError> {
            self.credentials.verify_token(token).await
        }
    }
}
