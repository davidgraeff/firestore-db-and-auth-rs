//! # Error and Result Type

use std::error;
use std::fmt;

use reqwest;

/// A result type that uses [`FirebaseError`] as an error type
pub type Result<T> = std::result::Result<T, FirebaseError>;

/// The main error type used throughout this crate. It wraps / converts from a few other error
/// types and implements [error::Error] so that you can use it in any situation where the
/// standard error type is expected.
#[derive(Debug)]
pub enum FirebaseError {
    Generic(&'static str),
    UnexpectedResponse(&'static str, reqwest::StatusCode, String, String),
    Request(reqwest::Error),
    JWT(biscuit::errors::Error),
    Ser(serde_json::Error),
    RSA(ring::error::KeyRejected),
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
        FirebaseError::Ser(error)
    }
}

impl std::convert::From<biscuit::errors::Error> for FirebaseError {
    fn from(error: biscuit::errors::Error) -> Self {
        FirebaseError::JWT(error)
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
            FirebaseError::Generic(ref m) => write!(f, "{}", m),
            FirebaseError::UnexpectedResponse(ref m, status, ref text, ref source) => {
                writeln!(f, "{} - {}", &m, status)?;
                writeln!(f, "{}", text)?;
                writeln!(f, "{}", source)?;
                Ok(())
            }
            FirebaseError::Request(ref e) => e.fmt(f),
            FirebaseError::JWT(ref e) => e.fmt(f),
            FirebaseError::RSA(ref e) => e.fmt(f),
            FirebaseError::IO(ref e) => e.fmt(f),
            //  FirebaseError::NoneError(ref e) => e.fmt(f),
            FirebaseError::Ser(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for FirebaseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FirebaseError::Generic(ref _m) => None,
            FirebaseError::UnexpectedResponse(_, _, _, _) => None,
            FirebaseError::Request(ref e) => Some(e),
            FirebaseError::JWT(ref e) => Some(e),
            FirebaseError::RSA(_) => None,
            FirebaseError::IO(ref e) => Some(e),
            //  FirebaseError::NoneError(ref e) => Some(e),
            FirebaseError::Ser(ref e) => Some(e),
        }
    }
}
