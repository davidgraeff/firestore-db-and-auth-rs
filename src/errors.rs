//! # Error and Result Type

use std::error;
use std::fmt;

use reqwest;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// A result type that uses [`FirebaseError`] as an error type
pub type Result<T> = std::result::Result<T, FirebaseError>;

/// The main error type used throughout this crate. It wraps / converts from a few other error
/// types and implements [error::Error] so that you can use it in any situation where the
/// standard error type is expected.
#[derive(Debug)]
pub enum FirebaseError {
    /// Generic errors are very rarely used and only used if no other error type matches
    Generic(&'static str),
    /// If the http status code is != 200 and no Google error response is attached
    /// (see https://firebase.google.com/docs/reference/rest/auth#section-error-format)
    /// then this error type will be returned
    UnexpectedResponse(&'static str, reqwest::StatusCode, String, String),
    /// An error returned by the Firestore API - Contains the numeric code, a string code and
    /// a context. If the APIError happened on a document query or mutation, the document
    /// path will be set as context.
    /// If the APIError happens on a user_* method, the user id will be set as context.
    /// For example: 400, CREDENTIAL_TOO_OLD_LOGIN_AGAIN
    APIError(usize, String, String),
    /// An error caused by the http library. This only happens if the http request is badly
    /// formatted (too big, invalid characters) or if the server did strange things
    /// (connection abort, ssl verification error).
    Request(reqwest::Error),
    /// Should not happen. If jwt encoding / decoding fails or an value cannot be extracted or
    /// a jwt is badly formatted or corrupted
    JWT(biscuit::errors::Error),
    JWTValidation(biscuit::errors::ValidationError),
    /// Serialisation failed
    Ser {
        doc: Option<String>,
        ser: serde_json::Error,
    },
    /// When the credentials.json file contains an invalid private key this error is returned
    RSA(ring::error::KeyRejected),
    /// Disk access errors
    IO(std::io::Error),
}

impl std::convert::From<std::io::Error> for FirebaseError {
    fn from(error: std::io::Error) -> Self {
        FirebaseError::IO(error)
    }
}

impl std::convert::From<ring::error::KeyRejected> for FirebaseError {
    fn from(error: ring::error::KeyRejected) -> Self {
        FirebaseError::RSA(error)
    }
}

impl std::convert::From<serde_json::Error> for FirebaseError {
    fn from(error: serde_json::Error) -> Self {
        FirebaseError::Ser { doc: None, ser: error }
    }
}

impl std::convert::From<biscuit::errors::Error> for FirebaseError {
    fn from(error: biscuit::errors::Error) -> Self {
        FirebaseError::JWT(error)
    }
}

impl std::convert::From<biscuit::errors::ValidationError> for FirebaseError {
    fn from(error: biscuit::errors::ValidationError) -> Self {
        FirebaseError::JWTValidation(error)
    }
}

impl std::convert::From<reqwest::Error> for FirebaseError {
    fn from(error: reqwest::Error) -> Self {
        FirebaseError::Request(error)
    }
}

impl fmt::Display for FirebaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FirebaseError::Generic(m) => write!(f, "{}", m),
            FirebaseError::APIError(code, ref m, ref context) => {
                write!(f, "API Error! Code {} - {}. Context: {}", code, m, context)
            }
            FirebaseError::UnexpectedResponse(m, status, ref text, ref source) => {
                writeln!(f, "{} - {}", &m, status)?;
                writeln!(f, "{}", text)?;
                writeln!(f, "{}", source)?;
                Ok(())
            }
            FirebaseError::Request(ref e) => e.fmt(f),
            FirebaseError::JWT(ref e) => e.fmt(f),
            FirebaseError::JWTValidation(ref e) => e.fmt(f),
            FirebaseError::RSA(ref e) => e.fmt(f),
            FirebaseError::IO(ref e) => e.fmt(f),
            FirebaseError::Ser { ref doc, ref ser } => {
                if let Some(doc) = doc {
                    writeln!(f, "{} in document {}", ser, doc)
                } else {
                    ser.fmt(f)
                }
            }
        }
    }
}

impl error::Error for FirebaseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FirebaseError::Generic(ref _m) => None,
            FirebaseError::UnexpectedResponse(_, _, _, _) => None,
            FirebaseError::APIError(_, _, _) => None,
            FirebaseError::Request(ref e) => Some(e),
            FirebaseError::JWT(ref e) => Some(e),
            FirebaseError::JWTValidation(ref e) => Some(e),
            FirebaseError::RSA(_) => None,
            FirebaseError::IO(ref e) => Some(e),
            FirebaseError::Ser { ref ser, .. } => Some(ser),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct GoogleRESTApiError {
    pub message: String,
    pub domain: String,
    pub reason: String,
}

#[derive(Default, Serialize, Deserialize)]
struct GoogleRESTApiErrorInfo {
    pub code: usize,
    pub message: String,
    pub errors: Option<Vec<GoogleRESTApiError>>,
}

#[derive(Default, Serialize, Deserialize)]
struct GoogleRESTApiErrorWrapper {
    pub error: Option<GoogleRESTApiErrorInfo>,
}

/// If the given reqwest response is status code 200, nothing happens
/// Otherwise the response will be analysed if it contains a Google API Error response.
/// See https://firebase.google.com/docs/reference/rest/auth#section-error-response
///
/// Arguments:
/// - response: The http requests response. Must be mutable, because the contained value will be extracted in an error case
/// - context: A function that will be called in an error case that returns a context string
pub(crate) async fn extract_google_api_error_async(
    response: reqwest::Response,
    context: impl Fn() -> String,
) -> Result<reqwest::Response> {
    if response.status() == 200 {
        return Ok(response);
    }

    Err(extract_google_api_error_intern(
        response.status().clone(),
        response.text().await?,
        context,
    ))
}

fn extract_google_api_error_intern(
    status: StatusCode,
    http_body: String,
    context: impl Fn() -> String,
) -> FirebaseError {
    let google_api_error_wrapper: std::result::Result<GoogleRESTApiErrorWrapper, serde_json::Error> =
        serde_json::from_str(&http_body);
    if let Ok(google_api_error_wrapper) = google_api_error_wrapper {
        if let Some(google_api_error) = google_api_error_wrapper.error {
            return FirebaseError::APIError(google_api_error.code, google_api_error.message.to_owned(), context());
        }
    };

    FirebaseError::UnexpectedResponse("", status, http_body, context())
}
