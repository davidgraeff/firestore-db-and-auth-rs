use crate::{errors::extract_google_api_error_async, FirebaseAuthBearerAsync};

use super::*;
use std::io::Read;

///
/// Read a document of a specific type from a collection by its Firestore document name
///
/// ## Arguments
/// * `auth` The authentication token
/// * `document_name` The document path / collection and document id; For example `projects/my_project/databases/(default)/documents/tests/test`
pub fn read_by_name<T>(auth: &impl FirebaseAuthBearer, document_name: impl AsRef<str>) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
{
    let resp = request_document(auth, document_name)?;
    // Here `resp.json()?` is a method provided by `reqwest`
    let json: dto::Document = resp.json()?;
    Ok(document_to_pod(&json)?)
}

///
/// Read a document of a specific type from a collection by its Firestore document name
///
/// ## Arguments
/// * `auth` The authentication token
/// * `document_name` The document path / collection and document id; For example `projects/my_project/databases/(default)/documents/tests/test`
pub async fn read_by_name_async<T>(auth: &mut impl FirebaseAuthBearerAsync, document_name: impl AsRef<str>) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
{
    let resp = request_document_async(auth, document_name).await?;
    // Here `resp.json()?` is a method provided by `reqwest`
    let json: dto::Document = resp.json().await?;
    Ok(document_to_pod(&json)?)
}

///
/// Read a document of a specific type from a collection
///
/// ## Arguments
/// * `auth` The authentication token
/// * `path` The document path / collection; For example `my_collection` or `a/nested/collection`
/// * `document_id` The document id. Make sure that you do not include the document id to the path argument.
pub fn read<T>(auth: &impl FirebaseAuthBearer, path: &str, document_id: impl AsRef<str>) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
{
    let document_name = document_name(&auth.project_id(), path, document_id);
    read_by_name(auth, &document_name)
}

/// Return the raw unparsed content of the Firestore document. Methods like
/// [`read()`](../documents/fn.read.html) will deserialize the JSON-encoded
/// response into a known type `T`
///
/// Note that this leverages [`std::io::Read`](https://doc.rust-lang.org/std/io/trait.Read.html) and the `read_to_string()` method to chunk the
/// response. This will raise `FirebaseError::IO` if there are errors reading the stream. Please
/// see [`read_to_end()`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end)
pub fn contents(auth: &impl FirebaseAuthBearer, path: &str, document_id: impl AsRef<str>) -> Result<String> {
    let document_name = document_name(&auth.project_id(), path, document_id);
    let mut resp = request_document(auth, document_name)?;
    let mut text = String::new();
    match resp.read_to_string(&mut text) {
        Ok(_bytes) => Ok(text),
        Err(e) => Err(FirebaseError::IO(e)),
    }
}

///
/// Read a document of a specific type from a collection
///
/// ## Arguments
/// * `auth` The authentication token
/// * `path` The document path / collection; For example `my_collection` or `a/nested/collection`
/// * `document_id` The document id. Make sure that you do not include the document id to the path argument.
pub async fn read_async<T>(
    auth: &mut impl FirebaseAuthBearerAsync,
    path: &str,
    document_id: impl AsRef<str>,
) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
{
    let document_name = document_name(&auth.project_id(), path, document_id);
    read_by_name_async(auth, &document_name).await
}

/// Return the raw unparsed content of the Firestore document. Methods like
/// [`read()`](../documents/fn.read.html) will deserialize the JSON-encoded
/// response into a known type `T`
///
/// Note that this leverages [`std::io::Read`](https://doc.rust-lang.org/std/io/trait.Read.html) and the `read_to_string()` method to chunk the
/// response. This will raise `FirebaseError::IO` if there are errors reading the stream. Please
/// see [`read_to_end()`](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end)
pub async fn contents_async(
    auth: &mut impl FirebaseAuthBearerAsync,
    path: &str,
    document_id: impl AsRef<str>,
) -> Result<String> {
    let document_name = document_name(&auth.project_id(), path, document_id);
    let resp = request_document_async(auth, document_name).await?;
    let mut text = String::new();
    let byte = match resp.bytes().await {
        Ok(_bytes) => _bytes,
        Err(e) => return Err(FirebaseError::Request(e)),
    };
    match std::str::from_utf8(&byte.to_vec()) {
        Ok(byte_str) => {
            text.push_str(byte_str);
        }
        Err(e) => return Err(FirebaseError::Utf8(e)),
    };
    Ok(text)
}

/// Executes the request to retrieve the document. Returns the response from `reqwest`
fn request_document(
    auth: &impl FirebaseAuthBearer,
    document_name: impl AsRef<str>,
) -> Result<reqwest::blocking::Response> {
    let url = firebase_url_base(document_name.as_ref());

    let resp = auth
        .client()
        .get(&url)
        .bearer_auth(auth.access_token().to_owned())
        .send()?;

    extract_google_api_error(resp, || document_name.as_ref().to_owned())
}

/// Executes the request to retrieve the document. Returns the response from `reqwest`
/// THIS IS A BLOCKING-OPERATION
async fn request_document_async(
    auth: &mut impl FirebaseAuthBearerAsync,
    document_name: impl AsRef<str>,
) -> Result<reqwest::Response> {
    let url = firebase_url_base(document_name.as_ref());

    let resp = auth
        .client_async()
        .get(&url)
        .bearer_auth(auth.access_token().await.to_string())
        .send()
        .await?;

    extract_google_api_error_async(resp, || document_name.as_ref().to_owned()).await
}

/// Simple method to join the path and document identifier in correct format
fn document_name(project_id: impl AsRef<str>, path: impl AsRef<str>, document_id: impl AsRef<str>) -> String {
    format!(
        "projects/{}/databases/(default)/documents/{}/{}",
        project_id.as_ref(),
        path.as_ref(),
        document_id.as_ref()
    )
}

#[test]
fn it_document_name_joins_paths() {
    let project_id = "firebase-project";
    let path = "one/two/three";
    let document_id = "my-document";
    assert_eq!(
        document_name(&project_id, &path, &document_id),
        "projects/firebase-project/databases/(default)/documents/one/two/three/my-document"
    );
}

#[test]
fn it_document_name_joins_invalid_path_fragments() {
    let project_id = "firebase-project";
    let path = "one/two//three/";
    let document_id = "///my-document";
    assert_eq!(
        document_name(&project_id, &path, &document_id),
        "projects/firebase-project/databases/(default)/documents/one/two//three/////my-document"
    );
}
