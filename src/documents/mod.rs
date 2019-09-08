//! # Firestore Document Access
//!
//! Interact with Firestore documents.
//! Please check the root page of this documentation for examples.

use super::dto;
use super::errors::{extract_google_api_error, FirebaseError, Result};
use super::firebase_rest_to_rust::{document_to_pod, pod_to_document};
use super::FirebaseAuthBearer;

use serde::{Deserialize, Serialize};
use std::path::Path;

mod list;
pub use list::*;

mod write;
pub use write::*;

mod query;
pub use query::*;

mod read;
pub use read::*;

/// An [`Iterator`] implementation that provides a join method
///
/// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
pub trait JoinableIterator: Iterator {
    fn join(&mut self, sep: &str) -> String
        where Self::Item: std::fmt::Display
    {
        use std::fmt::Write;
        match self.next() {
            None => String::new(),
            Some(first_elt) => {
                // estimate lower bound of capacity needed
                let (lower, _) = self.size_hint();
                let mut result = String::with_capacity(sep.len() * lower);
                write!(&mut result, "{}", first_elt).unwrap();
                for elt in self {
                    result.push_str(sep);
                    write!(&mut result, "{}", elt).unwrap();
                }
                result
            }
        }
    }
}

impl<'a, VALUE> JoinableIterator for std::collections::hash_map::Keys<'a, String, VALUE> {}


#[inline]
fn firebase_url_query(v1: &str) -> String {
    format!("https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents:runQuery", v1)
}

#[inline]
fn firebase_url_base(v1: &str) -> String {
    format!("https://firestore.googleapis.com/v1/{}", v1)
}

#[inline]
fn firebase_url_extended(v1: &str, v2: &str, v3: &str) -> String {
    format!("https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}", v1, v2, v3)
}

#[inline]
fn firebase_url(v1: &str, v2: &str) -> String {
    format!("https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?", v1, v2)
}

///
/// Deletes the document at the given path.
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The relative collection path and document id, for example "my_collection/document_id"
/// * 'fail_if_not_existing' If true this method will return an error if the document does not exist.
pub fn delete<'a, BEARER>(auth: &'a BEARER, path: &str, fail_if_not_existing: bool) -> Result<()>
    where
            for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let url = firebase_url(auth.project_id(), path);

    let query_request = dto::Write {
        current_document: Some(dto::Precondition {
            exists: match fail_if_not_existing {
                true => Some(true),
                false => None,
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut resp = auth.client()
        .delete(&url)
        .bearer_auth(auth.access_token().to_owned())
        .json(&query_request)
        .send()?;

    extract_google_api_error(&mut resp, || path.to_owned())?;

    Ok({})
}
