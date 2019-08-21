use super::credentials;
use super::errors;

use super::FirebaseAuthBearer;

pub mod user {
    use super::errors::{FirebaseError, Result};

    use biscuit::jwa::SignatureAlgorithm;
    use biscuit::{
        jws::{RegisteredHeader, Secret},
        ClaimsSet, RegisteredClaims, SingleOrMultiple, JWT,
    };

    use super::credentials::Credentials;

    use chrono::{Duration, Utc};
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use std::str::FromStr;

    use std::ops::Add;

    static JWT_AUD: &str =
        "https://identitytoolkit.googleapis.com/google.identity.identitytoolkit.v1.IdentityToolkit";

    macro_rules! token_endpoint {
        () => {
            "https://www.googleapis.com/identitytoolkit/v3/relyingparty/verifyCustomToken?key={}"
        };
    }
    macro_rules! refresh_to_access_endpoint {
        () => {
            "https://securetoken.googleapis.com/v1/token?key={}"
        };
    }

    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    pub struct JWTGoogleClaims {
        scope: String,
    }

    pub type FirebaseIDTokenJWT = JWT<JWTGoogleClaims, biscuit::Empty>;

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
        fn bearer(&'a mut self) -> &'a str {
            &self.bearer
        }
    }

    #[allow(non_snake_case)]
    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    struct CustomJwtToFirebaseID {
        token: String,
        returnSecureToken: bool,
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

    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    pub struct FirebaseUIDClaims {
        uid: String,
    }

    impl Session {
        /// Create a new firestore user session via a valid refresh_token
        pub fn by_refresh_token(credentials: &Credentials, refresh_token: &str) -> Result<Session> {
            let request_body = vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ];

            let url = format!(refresh_to_access_endpoint!(), credentials.api_key);
            let client = Client::new();
            let ref mut response = client.post(&url).form(&request_body).send()?;
            let r: RefreshTokenToAccessTokenResponse = response.json()?;
            return Ok(Session {
                userid: r.user_id,
                bearer: r.id_token,
                refresh_token: Some(r.refresh_token),
                projectid: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
            });
        }

        /// Create a new firestore user session with a fresh access token and new refresh token
        pub fn by_user_id(credentials: &Credentials, user_id: &str) -> Result<Session> {
            // Create custom jwt
            let header = From::from(RegisteredHeader {
                algorithm: SignatureAlgorithm::RS256,
                key_id: Some(credentials.private_key_id.to_owned()),
                ..Default::default()
            });
            let expected_claims = ClaimsSet::<FirebaseUIDClaims> {
                registered: RegisteredClaims {
                    issuer: Some(FromStr::from_str(&credentials.client_email)?),
                    subject: Some(FromStr::from_str(&credentials.client_email)?),
                    audience: Some(SingleOrMultiple::Single(FromStr::from_str(JWT_AUD)?)),
                    expiry: Some(biscuit::Timestamp::from(Utc::now().add(Duration::hours(1)))),
                    issued_at: Some(biscuit::Timestamp::from(Utc::now())),
                    ..Default::default()
                },
                private: FirebaseUIDClaims {
                    uid: user_id.to_owned(),
                },
            };
            let jwt = JWT::new_decoded(header, expected_claims);

            let signing_secret =
                Secret::RsaKeyPair(std::sync::Arc::new(credentials.create_rsa_key_pair()?));
            let jwt = jwt
                .into_encoded(&signing_secret)?
                .unwrap_encoded()
                .to_string();

            let url = format!(token_endpoint!(), credentials.api_key);
            let json = CustomJwtToFirebaseID {
                returnSecureToken: true,
                token: jwt,
            };

            let client = Client::new();
            let mut r = client.post(&url).json(&json).send()?;
            if r.status() != 200 {
                return Err(FirebaseError::UnexpectedResponse(
                    "Server responded with an error",
                    r.status(),
                    r.text()?,
                    serde_json::to_string_pretty(&json)?,
                ));
            }
            let r: CustomJwtToFirebaseIDResponse = r.json()?;

            Ok(Session {
                userid: user_id.to_owned(),
                bearer: r.idToken,
                refresh_token: Some(r.refreshToken),
                projectid: credentials.project_id.to_owned(),
                api_key: credentials.api_key.clone(),
            })
        }

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
        ) -> Result<Session> {
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

        pub fn by_access_token(
            credentials: &Credentials,
            firebase_tokenid: &str,
        ) -> Result<Session> {
            let token = JWT::<biscuit::Empty, biscuit::Empty>::new_encoded(&firebase_tokenid);

            let header = token.unverified_header()?;
            let kid = header
                .registered
                .key_id
                .as_ref()
                .ok_or(FirebaseError::Generic("No key ID"))?;

            let key_params = credentials
                .public_key(kid)
                .ok_or(FirebaseError::Generic("Did not find jwk"))?;
            let secret = key_params.jws_public_key_secret();

            let token = token.decode(&secret, SignatureAlgorithm::RS256)?;

            // Check expire time and issued-at time
            let ref claims = token.payload()?.registered;
            if let Some(issued_at) = claims.issued_at.as_ref() {
                if Utc::now().time() < issued_at.time() {
                    return Err(FirebaseError::Generic("Token has invalid issued_at"));
                }
            }
            if let Some(expiry) = claims.expiry.as_ref() {
                if Utc::now().time() > expiry.time() {
                    return Err(FirebaseError::Generic("Token expired"));
                }
            }

            let userid = claims.subject.as_ref().map(|ref f| f.to_string()).ok_or(
                FirebaseError::Generic("Subject not set. Can not extract userid"),
            )?;
            let projectid = claims.audience.as_ref().ok_or(FirebaseError::Generic(
                "Audience not set. Can not extract projectid",
            ))?;

            if let SingleOrMultiple::Single(projectid) = projectid {
                return Ok(Session {
                    userid,
                    projectid: projectid.to_string(),
                    bearer: firebase_tokenid.to_owned(),
                    refresh_token: None,
                    api_key: credentials.api_key.clone(),
                });
            }
            return Err(FirebaseError::Generic(
                "jwk: Haven't found all required fields",
            ));
        }
    }
}

/// Find the service account session defined in here
pub mod service_account {
    use biscuit::jwa::SignatureAlgorithm;
    use biscuit::{
        jws::{RegisteredHeader, Secret},
        ClaimsSet, RegisteredClaims, SingleOrMultiple, StringOrUri, JWT,
    };


    use serde::{Deserialize, Serialize};
    use std::str::FromStr;
    pub type GoogleJWT = JWT<biscuit::Empty, biscuit::Empty>;

    static JWT_SUBJECT: &str = "https://firestore.googleapis.com/google.firestore.v1.Firestore";

    use chrono::{Duration, Utc};

    use std::ops::Add;

    use super::credentials::Credentials;
    use super::errors::Result;

    /// Service account session
    #[derive(Serialize, Deserialize)]
    pub struct Session {
        pub credentials: Credentials,
        jwt: GoogleJWT,
        bearer_cache: String,
    }

    impl<'a> super::FirebaseAuthBearer<'a> for Session {
        fn projectid(&'a self) -> &'a str {
            &self.credentials.project_id
        }
        /// Return the encoded jwt to be used as bearer token. If the jwt
        /// issue_at is older than 50 mins, it will be updated to the current time.
        ///
        /// For this to work, you must use a mutable session object
        fn bearer(&'a mut self) -> &'a str {
            let ref mut claims = self.jwt.payload_mut().unwrap().registered;

            let now = biscuit::Timestamp::from(Utc::now());
            if let Some(issued_at) = claims.issued_at.as_ref() {
                let diff: Duration = Utc::now().time() - issued_at.time();
                if diff.num_minutes() > 50 {
                    claims.issued_at = Some(now);
                } else {
                    return &self.bearer_cache;
                }
            } else {
                claims.issued_at = Some(now);
            }
            let signing_secret = Secret::RsaKeyPair(std::sync::Arc::new(
                self.credentials.create_rsa_key_pair().unwrap(),
            ));
            self.bearer_cache = self
                .jwt
                .encode(&signing_secret)
                .unwrap()
                .unwrap_encoded()
                .encode();
            return &self.bearer_cache;
        }
    }

    #[cfg(feature = "faststart")]
    #[macro_export]
    macro_rules! from_binary {
        ($filename:expr) => {{
            let bytes = include_bytes!($filename);
            bincode::deserialize(bytes).unwrap()
        };};
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
        pub fn new(credentials: Credentials) -> Result<Session> {
            let header = From::from(RegisteredHeader {
                algorithm: SignatureAlgorithm::RS256,
                key_id: Some(credentials.private_key_id.to_owned()),
                ..Default::default()
            });
            let expected_claims = ClaimsSet::<biscuit::Empty> {
                // JWTGoogleClaims
                registered: RegisteredClaims {
                    issuer: Some(FromStr::from_str(&credentials.client_email[..])?),
                    audience: Some(SingleOrMultiple::Single(StringOrUri::from_str(
                        JWT_SUBJECT,
                    )?)),
                    subject: Some(StringOrUri::from_str(&credentials.client_email[..])?),
                    expiry: Some(biscuit::Timestamp::from(Utc::now().add(Duration::hours(1)))),
                    issued_at: Some(biscuit::Timestamp::from(Utc::now())),
                    ..Default::default()
                },
                ..Default::default()
            };
            let jwt = JWT::new_decoded(header, expected_claims);

            let signing_secret =
                Secret::RsaKeyPair(std::sync::Arc::new(credentials.create_rsa_key_pair()?));
            Ok(Session {
                bearer_cache: jwt.encode(&signing_secret)?.unwrap_encoded().encode(),
                jwt: jwt,
                credentials: credentials,
            })
        }

        pub fn from_binary(binary_session_file: &str) -> Result<Session> {
            use std::fs::File;
            use std::io::prelude::*;
            use std::path::Path;
            let mut target = File::open(&Path::new(binary_session_file))?;
            let mut data = Vec::new();
            target.read_to_end(&mut data)?;

            let mut credentials: Credentials = bincode::deserialize(&data)?;
            credentials.compute_missing_fields()?;
            Session::new(credentials)
        }
    }
}
