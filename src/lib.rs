#![deny(warnings)]
#![cfg_attr(feature = "external_doc", feature(external_doc))]
#![cfg_attr(feature = "external_doc", doc(include = "../readme.md"))]

extern crate regex;
extern crate ring;
extern crate untrusted;

pub mod credentials;
pub mod documents;
pub(crate) mod dto;
pub mod errors;
pub mod firebase_rest_to_rust;
pub mod jwt;
pub mod sessions;
pub mod users;

#[doc(hidden)]
pub mod private {
    pub use crate::dto::*;
}

#[cfg(feature = "rocket_support")]
pub mod rocket;

// Forward declarations
pub use credentials::Credentials;

/// Authentication trait.
///
/// This trait is implemented by [`crate::sessions`], but you do not need those for using the Firestore API of this crate.
/// Firestore document methods in [`crate::documents`] expect an object that implements the `FirebaseAuthBearer` trait.
///
/// Implement this trait for your own data structure and provide the Firestore project id and a valid access token.
pub trait FirebaseAuthBearer<'a> {
    /// Return the project ID. This is required for the firebase REST API.
    fn projectid(&'a self) -> &'a str;
    /// An access token, preferably valid.
    fn bearer(&'a self) -> String;
}
