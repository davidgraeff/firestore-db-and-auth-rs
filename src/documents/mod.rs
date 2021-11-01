//! # Firestore Document Access
//!
//! Interact with Firestore documents.
//! Please check the root page of this documentation for examples.
#![allow(unused_imports, dead_code)]
use super::dto;
use super::errors::{extract_google_api_error, extract_google_api_error_async, FirebaseError, Result};
use super::firebase_rest_to_rust::{document_to_pod, pod_to_document};
use super::FirebaseAuthBearer;

use serde::{Deserialize, Serialize};
use std::path::Path;

mod delete;
mod read;
mod write;
mod list;
mod query;

pub use delete::*;
pub use read::*;
pub use write::*;
pub use list::*;
pub use query::*;


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
