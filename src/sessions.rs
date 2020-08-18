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
    use std::time::{SystemTime, UNIX_EPOCH};
    use jsonwebtoken::{Algorithm, encode, EncodingKey, Header};

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
        /// If the refresh failed, this will
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

    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
    #[allow(non_snake_case)]
    struct SessionLogin {
        idToken: String,
        validDuration: u64,
    }

    #[derive(Debug, Deserialize)]
    struct Oauth2Response {
        access_token: String,
        expires_in: u64,
        token_type: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        scope: String,
        aud: String,
        exp: u64,
        iat: u64,
        iss: String,
    }

    #[derive(Debug, Deserialize)]
    #[allow(non_snake_case)]
    struct CreateSessionCookieResponseJson {
        sessionCookie: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateSessionCookie {
        pub session_cookie: String,
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

        pub fn by_access_token(credentials: &Credentials, firebase_tokenid: &str) -> Result<Session, FirebaseError> {
            let result = verify_access_token(&credentials, firebase_tokenid)?;
            Ok(Session {
                user_id: result.subject,
                project_id_: result.audience,
                access_token_: RefCell::new(firebase_tokenid.to_owned()),
                refresh_token: None,
                api_key: credentials.api_key.clone(),
                client: reqwest::blocking::Client::new(),
                client_async: reqwest::Client::new(),
            })
        }

        pub fn create_session_cookie(credentials: &Credentials, id_token: String, duration: u64) -> Result<CreateSessionCookie, FirebaseError> {
            let scope_list = vec![
                "https://www.googleapis.com/auth/cloud-platform".to_string(),
                "https://www.googleapis.com/auth/firebase.database".to_string(),
                "https://www.googleapis.com/auth/firebase.messaging".to_string(),
                "https://www.googleapis.com/auth/identitytoolkit".to_string(),
                "https://www.googleapis.com/auth/userinfo.email".to_string(),
            ];

            // Generate the assertion from the admin credentials
            let iat  = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let assertion = encode(&Header::new(Algorithm::RS256),
                                   &Claims {
                                       scope: scope_list.join(" "),
                                       aud: "https://accounts.google.com/o/oauth2/token".to_string(),
                                       iat,
                                       exp : iat + duration,
                                       iss: credentials.client_email.to_string(),
                                   },
                                   &EncodingKey::from_rsa_pem(credentials.private_key.to_string().as_ref()).unwrap())
                .unwrap();

            // Request Google Oauth2 to retrieve the access token in order to create a session cookie
            let client = reqwest::blocking::Client::new();
            let google_oauth2_url = "https://accounts.google.com/o/oauth2/token".to_string();
            let response_oauth2: Oauth2Response = client.post(&google_oauth2_url)
                .form(&[("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"), ("assertion", &assertion)])
                .send()
                .unwrap()
                .json()
                .unwrap();

            // Create a session cookie with the access token previously retrieved
            let create_session_cookie_url: String = "https://identitytoolkit.googleapis.com/v1/projects/arenel:createSessionCookie".to_string();
            let response_session_cookie_json: CreateSessionCookieResponseJson = client.post(&create_session_cookie_url)
                .bearer_auth(&response_oauth2.access_token)
                .json(&SessionLogin {
                    idToken: id_token,
                    validDuration: duration,
                })
                .send()
                .unwrap()
                .json()
                .unwrap();


            Ok(CreateSessionCookie {
                session_cookie : response_session_cookie_json.sessionCookie
            })
        }
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
