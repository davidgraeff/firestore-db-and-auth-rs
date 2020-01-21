//! # Firestore Document Access
//!
//! Interact with Firestore documents.
//! Please check the root page of this documentation for examples.

use super::dto;
use super::errors::{extract_google_api_response, FirebaseError, Result};
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
        where
            Self::Item: std::fmt::Display,
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
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents:runQuery",
        v1
    )
}

#[inline]
fn firebase_url_base(v1: &str) -> String {
    format!("https://firestore.googleapis.com/v1/{}", v1)
}

#[inline]
fn firebase_url_extended(v1: &str, v2: &str, v3: &str) -> String {
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
        v1, v2, v3
    )
}

#[inline]
fn firebase_url(v1: &str, v2: &str) -> String {
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?",
        v1, v2
    )
}

/// Converts an absolute path like "projects/{PROJECT_ID}/databases/(default)/documents/my_collection/document_id"
/// into a relative document path like "my_collection/document_id"
///
/// This is usually used to get a suitable path for [`delete`].
pub fn abs_to_rel(path: &str) -> &str {
    &path[path.find("(default)").unwrap() + 20..]
}

#[test]
fn abs_to_rel_test() {
    assert_eq!(
        abs_to_rel("projects/{PROJECT_ID}/databases/(default)/documents/my_collection/document_id"),
        "my_collection/document_id"
    );
}

///
/// Deletes the document at the given path.
///
/// You cannot use this directly with paths from [`list`] and [`query`] document metadata objects.
/// Those contain an absolute document path. Use [`abs_to_rel`] to convert to a relative path.
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The relative collection path and document id, for example "my_collection/document_id"
/// * 'fail_if_not_existing' If true this method will return an error if the document does not exist.
pub fn delete(auth: &impl FirebaseAuthBearer, path: &str, fail_if_not_existing: bool) -> Result<()> {
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

    let resp = auth
        .client()
        .delete(&url)
        .bearer_auth(auth.access_token().to_owned())
        .json(&query_request)
        .send()?;

    extract_google_api_response(resp, || path.to_owned())
        .map(|_resp: serde_json::value::Value| ())
}
