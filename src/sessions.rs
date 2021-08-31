//! # Authentication Session - Contains non-persistent access tokens
//!
//! A session can be either for a service-account or impersonated via a firebase auth user id.

use super::credentials;
use super::errors::{extract_google_api_error, FirebaseError};
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

pub mod user {
    use super::*;
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
        /// The http client. Replace or modify the client if you have special demands like proxy support
        pub client: reqwest::blocking::Client,
        /// The http client for async operations. Replace or modify the client if you have special demands like proxy support
        pub client_async: reqwest::Client,
    }

    impl super::FirebaseAuthBearer for Session {
        fn project_id(&self) -> &str {
            &self.project_id_
        }
        /// Returns the current access token.
        /// This method will automatically refresh your access token, if it has expired.
        ///
        /// If the refresh failed, this will return an empty string.
        fn access_token(&self) -> String {
            let jwt = self.access_token_.borrow();
            let jwt = jwt.as_str();

            if is_expired(&jwt, 0).unwrap() {
                // Unwrap: the token is always valid at this point
                if let Ok(response) = get_new_access_token(&self.api_key, jwt) {
                    self.access_token_.swap(&RefCell::new(response.id_token.clone()));
                    return response.id_token;
                } else {
                    // Failed to refresh access token. Return an empty string
                    return String::new();
                }
            }
            jwt.to_owned()
        }

        fn access_token_unchecked(&self) -> String {
            self.access_token_.borrow().clone()
        }

        fn client(&self) -> &reqwest::blocking::Client {
            &self.client
        }

        fn client_async(&self) -> &reqwest::Client {
            &self.client_async
        }
    }

    /// Gets a new access token via an api_key and a refresh_token.
    /// This is a blocking operation.
    fn get_new_access_token(
        api_key: &str,
        refresh_token: &str,
    ) -> Result<RefreshTokenToAccessTokenResponse, FirebaseError> {
        let request_body = vec![("grant_type", "refresh_token"), ("refresh_token", refresh_token)];

        let url = refresh_to_access_endpoint(api_key);
        let client = reqwest::blocking::Client::new();
        let response = client.post(&url).form(&request_body).send()?;
        Ok(response.json()?)
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
                    let mut r = r.unwrap();
                    r.refresh_token = refresh_token.and_then(|f| Some(f.to_owned()));
                    return Ok(r);
                }
            }

            // Check if refresh_token is already sufficient
            if let Some(refresh_token) = refresh_token {
                let r = Session::by_refresh_token(credentials, refresh_token);
                if r.is_ok() {
                    return r;
                }
            }

            // Neither refresh token nor access token worked or are provided.
            // Try to get new new tokens for the given user_id via the REST API and the service-account credentials.
            if let Some(user_id) = user_id {
                let r = Session::by_user_id(credentials, user_id, true);
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
        pub fn by_refresh_token(credentials: &Credentials, refresh_token: &str) -> Result<Session, FirebaseError> {
            let r: RefreshTokenToAccessTokenResponse = get_new_access_token(&credentials.api_key, refresh_token)?;
            Ok(Session {
                user_id: r.user_id,
                access_token_: RefCell::new(r.id_token),
                refresh_token: Some(r.refresh_token),
                project_id_: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
                client: reqwest::blocking::Client::new(),
                client_async: reqwest::Client::new(),
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
        pub fn by_user_id(
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
            let secret = credentials
                .keys
                .secret
                .as_ref()
                .ok_or(FirebaseError::Generic("No private key added via add_keypair_key!"))?;
            let encoded = jwt.encode(&secret.deref())?.encoded()?.encode();

            let resp = reqwest::blocking::Client::new()
                .post(&token_endpoint(&credentials.api_key))
                .json(&CustomJwtToFirebaseID::new(encoded, with_refresh_token))
                .send()?;
            let resp = extract_google_api_error(resp, || user_id.to_owned())?;
            let r: CustomJwtToFirebaseIDResponse = resp.json()?;

            Ok(Session {
                user_id: user_id.to_owned(),
                access_token_: RefCell::new(r.idToken),
                refresh_token: r.refreshToken,
                project_id_: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
                client: reqwest::blocking::Client::new(),
                client_async: reqwest::Client::new(),
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
        pub fn by_access_token(credentials: &Credentials, access_token: &str) -> Result<Session, FirebaseError> {
            let result = verify_access_token(&credentials, access_token)?;
            Ok(Session {
                user_id: result.subject,
                project_id_: result.audience,
                access_token_: RefCell::new(access_token.to_owned()),
                refresh_token: None,
                api_key: credentials.api_key.clone(),
                client: reqwest::blocking::Client::new(),
                client_async: reqwest::Client::new(),
            })
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
    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
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
        expires_in: u64,
        token_type: String,
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
    pub fn create(
        credentials: &credentials::Credentials,
        id_token: String,
        duration: chrono::Duration,
    ) -> Result<String, FirebaseError> {
        // Generate the assertion from the admin credentials
        let assertion = crate::jwt::session_cookie::create_jwt_encoded(credentials, chrono::Duration::minutes(1))?;

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
    use super::*;
    use credentials::Credentials;

    use chrono::Duration;
    use std::cell::RefCell;
    use std::ops::Deref;

    /// Service account session
    pub struct Session {
        /// The google credentials
        pub credentials: Credentials,
        /// The http client. Replace or modify the client if you have special demands like proxy support
        pub client: reqwest::blocking::Client,
        /// The http client for async operations. Replace or modify the client if you have special demands like proxy support
        pub client_async: reqwest::Client,
        jwt: RefCell<AuthClaimsJWT>,
        access_token_: RefCell<String>,
    }

    impl super::FirebaseAuthBearer for Session {
        fn project_id(&self) -> &str {
            &self.credentials.project_id
        }
        /// Return the encoded jwt to be used as bearer token. If the jwt
        /// issue_at is older than 50 minutes, it will be updated to the current time.
        fn access_token(&self) -> String {
            let mut jwt = self.jwt.borrow_mut();

            if jwt_update_expiry_if(&mut jwt, 50) {
                if let Some(secret) = self.credentials.keys.secret.as_ref() {
                    if let Ok(v) = self.jwt.borrow().encode(&secret.deref()) {
                        if let Ok(v2) = v.encoded() {
                            self.access_token_.swap(&RefCell::new(v2.encode()));
                        }
                    }
                }
            }

            self.access_token_.borrow().clone()
        }

        fn access_token_unchecked(&self) -> String {
            self.access_token_.borrow().clone()
        }

        fn client(&self) -> &reqwest::blocking::Client {
            &self.client
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
                Duration::hours(1),
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
                client: reqwest::blocking::Client::new(),
                client_async: reqwest::Client::new(),
            })
        }
    }
}
