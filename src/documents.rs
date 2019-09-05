//! # Firestore Document Access
//!
//! Interact with Firestore documents.
//! Please check the root page of this documentation for examples.

macro_rules! firebase_url_query {
    () => {
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents:runQuery"
    };
}
macro_rules! firebase_url_base {
    () => {
        "https://firestore.googleapis.com/v1/{}"
    };
}
macro_rules! firebase_url_extended {
    () => {
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}"
    };
}
macro_rules! firebase_url {
    () => {
        "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?"
    };
}

use super::dto;
use super::errors::{extract_google_api_error, FirebaseError, Result};
use super::firebase_rest_to_rust::{document_to_pod, pod_to_document};
use super::FirebaseAuthBearer;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// This is returned by the write() method in a successful case.
///
/// This structure contains the document id of the written document.
#[derive(Serialize, Deserialize)]
pub struct WriteResult {
    ///
    pub create_time: Option<chrono::DateTime<chrono::Utc>>,
    pub update_time: Option<chrono::DateTime<chrono::Utc>>,
    pub document_id: String,
}

///
/// Write a document to a given collection.
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The document path / collection; For example "my_collection" or "a/nested/collection"
/// * 'document_id' The document id. Make sure that you do not include the document id to the path argument.
/// * 'document' The document
pub fn write<'a, T, BEARER>(
    auth: &'a BEARER,
    path: &str,
    document_id: Option<&str>,
    document: &T,
) -> Result<WriteResult>
where
    T: Serialize,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let url = match document_id {
        Some(document_id) => format!(
            firebase_url_extended!(),
            auth.projectid(),
            path,
            document_id
        ),
        None => format!(firebase_url!(), auth.projectid(), path),
    };

    let firebase_document = pod_to_document(&document)?;

    let builder = if document_id.is_some() {
        Client::new().patch(&url)
    } else {
        Client::new().post(&url)
    };

    let mut resp = builder
        .bearer_auth(auth.bearer().to_owned())
        .json(&firebase_document)
        .send()?;

    extract_google_api_error(&mut resp, || document_id.or(Some("")).unwrap().to_owned())?;

    let result_document: dto::Document = resp.json()?;
    let doc_path = result_document.name.ok_or_else(|| {
        FirebaseError::Generic("Resulting document does not contain a 'name' field")
    })?;
    let document_id = Path::new(&doc_path)
        .file_name()
        .ok_or_else(|| {
            FirebaseError::Generic("Resulting documents 'name' field is not a valid path")
        })?
        .to_str()
        .ok_or_else(|| FirebaseError::Generic("No valid unicode in 'name' field"))?
        .to_owned();

    let create_time = match result_document.create_time {
        Some(f) => Some(
            chrono::DateTime::parse_from_rfc3339(&f)
                .map_err(|_| {
                    FirebaseError::Generic("Failed to parse rfc3339 date from 'create_time' field")
                })?
                .with_timezone(&chrono::Utc),
        ),
        None => None,
    };
    let update_time = match result_document.update_time {
        Some(f) => Some(
            chrono::DateTime::parse_from_rfc3339(&f)
                .map_err(|_| {
                    FirebaseError::Generic("Failed to parse rfc3339 date from 'update_time' field")
                })?
                .with_timezone(&chrono::Utc),
        ),
        None => None,
    };

    Ok(WriteResult {
        document_id,
        create_time,
        update_time,
    })
}

///
/// Read a document of a specific type from a collection by its Firestore document name
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'document_name' The document path / collection and document id; For example "projects/my_project/databases/(default)/documents/tests/test"
pub fn read_by_name<'a, T, BEARER>(auth: &'a BEARER, document_name: &str) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let url = format!(firebase_url_base!(), document_name);

    let mut resp = Client::new()
        .get(&url)
        .bearer_auth(auth.bearer().to_owned())
        .send()?;

    extract_google_api_error(&mut resp, || document_name.to_owned())?;

    let json: dto::Document = resp.json()?;
    Ok(document_to_pod(&json)?)
}

///
/// Read a document of a specific type from a collection
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The document path / collection; For example "my_collection" or "a/nested/collection"
/// * 'document_id' The document id. Make sure that you do not include the document id to the path argument.
pub fn read<'a, T, BEARER>(auth: &'a BEARER, path: &str, document_id: &str) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let document_name = format!(
        "projects/{}/databases/(default)/documents/{}/{}",
        auth.projectid(),
        path,
        document_id
    );
    read_by_name(auth, &document_name)
}

/// Use this type to list all documents of a given collection.
///
/// Please note that this API acts as an iterator of same-like documents.
/// This type is not suitable if you want to list documents of different types.
pub struct List<'a, T, BEARER> {
    auth: &'a BEARER,
    next_page_token: Option<String>,
    documents: Vec<dto::Document>,
    current: usize,
    done: bool,
    url: String,
    collection_id: String,
    phantom: std::marker::PhantomData<T>,
}

/// List all documents of a given collection.
///
/// Please note that this API acts as an iterator of same-like documents.
/// This type is not suitable if you want to list documents of different types.
///
/// Example:
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize)]
/// struct DemoDTO { a_string: String, an_int: u32, }
///
/// use firestore_db_and_auth::{documents};
/// # use firestore_db_and_auth::{credentials::Credentials, sessions, errors::Result};
///
/// # let credentials : Credentials = Credentials::new(include_str!("../firebase-service-account.json"),
///                                         &[include_str!("../tests/service-account-for-tests.jwks")])?;
/// # let session = sessions::service_account::Session::new(credentials)?;
///
/// let values: documents::List<DemoDTO, _> = documents::list(&session, "tests".to_owned());
/// for doc_result in values {
///     // The data is wrapped in a Result<> because fetching new data could have failed
///     // A tuple is returned on success with the document itself and and metadata
///     // with .name, .create_time, .update_time fields.
///     let (doc, _metadata) = doc_result?;
///     println!("{:?}", doc);
/// }
/// # Ok::<(), firestore_db_and_auth::errors::FirebaseError>(())
/// ```
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'collection_id' The document path / collection; For example "my_collection" or "a/nested/collection"
pub fn list<'a, T, BEARER>(
    auth: &'a BEARER,
    collection_id: impl Into<String>,
) -> List<'a, T, BEARER>
where
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let collection_id = collection_id.into();
    List {
        url: format!(firebase_url!(), auth.projectid(), collection_id),
        auth,
        next_page_token: None,
        documents: vec![],
        current: 0,
        done: false,
        collection_id,
        phantom: std::marker::PhantomData,
    }
}

fn get_new_data<'a>(
    collection_id: &str,
    url: &str,
    auth: &'a dyn FirebaseAuthBearer<'a>,
) -> Result<dto::ListDocumentsResponse> {
    let mut resp = Client::new()
        .get(url)
        .bearer_auth(auth.bearer().to_owned())
        .send()?;

    extract_google_api_error(&mut resp, || collection_id.to_owned())?;

    let json: dto::ListDocumentsResponse = resp.json()?;
    Ok(json)
}

impl<'a, T, BEARER> Iterator for List<'a, T, BEARER>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    type Item = Result<(T, dto::Document)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.documents.len() <= self.current {
            let url = match &self.next_page_token {
                Some(next_page_token) => format!("{}pageToken={}", self.url, next_page_token),
                None => self.url.clone(),
            };

            let result = get_new_data(&self.collection_id, &url, self.auth);
            match result {
                Err(e) => {
                    self.done = true;
                    return Some(Err(e));
                }
                Ok(v) => match v.documents {
                    None => return None,
                    Some(documents) => {
                        self.documents = documents;
                        self.current = 0;
                        self.next_page_token = v.next_page_token;
                    }
                },
            };
        }

        let doc = self.documents.get(self.current).unwrap();

        self.current += 1;
        if self.documents.len() <= self.current && self.next_page_token.is_none() {
            self.done = true;
        }

        let result = document_to_pod(&doc);
        match result {
            Err(e) => Some(Err(e)),
            Ok(pod) => Some(Ok((
                pod,
                dto::Document {
                    update_time: doc.update_time.clone(),
                    create_time: doc.create_time.clone(),
                    name: doc.name.clone(),
                    fields: None,
                },
            ))),
        }
    }
}

///
/// Queries the database for specific documents, for example all documents in a collection of the 'type' == "car".
///
/// Please note that this API returns a vector of same-like documents.
/// This method is not suitable if you want to query for different types of documents.
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'collectionid' The collection id; "my_collection" or "a/nested/collection"
/// * 'value' The query / filter value. For example "car".
/// * 'operator' The query operator. For example "EQUAL".
/// * 'field' The query / filter field. For example "type".
pub fn query<'a, T, BEARER>(
    auth: &'a BEARER,
    collectionid: &str,
    value: &str,
    operator: dto::FieldOperator,
    field: &str,
) -> Result<Vec<T>>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let url = format!(firebase_url_query!(), auth.projectid());

    let query_request = dto::RunQueryRequest {
        structured_query: Some(dto::StructuredQuery {
            select: Some(dto::Projection { fields: None }),
            where_: Some(dto::Filter {
                field_filter: Some(dto::FieldFilter {
                    value: dto::Value {
                        string_value: Some(value.to_owned()),
                        ..Default::default()
                    },
                    op: operator,
                    field: dto::FieldReference {
                        field_path: field.to_owned(),
                    },
                }),
                ..Default::default()
            }),
            from: Some(vec![dto::CollectionSelector {
                collection_id: Some(collectionid.to_owned()),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut resp = Client::new()
        .post(&url)
        .bearer_auth(auth.bearer().to_owned())
        .json(&query_request)
        .send()?;

    extract_google_api_error(&mut resp, || collectionid.to_owned())?;

    let json: Option<Vec<dto::RunQueryResponse>> = resp.json()?;

    let mut dtos: Vec<T> = Vec::new();
    if json.is_none() {
        return Ok(dtos);
    }
    let json = json.unwrap();

    for value in json.iter() {
        if let Some(ref document) = &value.document {
            if document.fields.is_none() && document.name.is_some() {
                let doc: T = read_by_name(auth, &document.name.as_ref().unwrap())?;
                dtos.push(doc);
            } else {
                dtos.push(document_to_pod(document)?);
            }
        }
    }
    Ok(dtos)
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
    let url = format!(firebase_url!(), auth.projectid(), path);

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

    let mut resp = Client::new()
        .delete(&url)
        .bearer_auth(auth.bearer().to_owned())
        .json(&query_request)
        .send()?;

    extract_google_api_error(&mut resp, || path.to_owned())?;

    Ok({})
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::Result;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct DemoPod {
        integer_test: u32,
        boolean_test: bool,
        string_test: String,
    }

    #[test]
    fn test_document_to_pod() -> Result<()> {
        let mut map: HashMap<String, dto::Value> = HashMap::new();
        map.insert(
            "integer_test".to_owned(),
            dto::Value {
                integer_value: Some("12".to_owned()),
                ..Default::default()
            },
        );
        map.insert(
            "boolean_test".to_owned(),
            dto::Value {
                boolean_value: Some(true),
                ..Default::default()
            },
        );
        map.insert(
            "string_test".to_owned(),
            dto::Value {
                string_value: Some("abc".to_owned()),
                ..Default::default()
            },
        );
        let t = dto::Document {
            fields: Some(map),
            ..Default::default()
        };
        let firebase_doc: DemoPod = document_to_pod(&t)?;
        assert_eq!(firebase_doc.string_test, "abc");
        assert_eq!(firebase_doc.integer_test, 12);
        assert_eq!(firebase_doc.boolean_test, true);

        Ok(())
    }

    #[test]
    fn test_pod_to_document() -> Result<()> {
        let t = DemoPod {
            integer_test: 12,
            boolean_test: true,
            string_test: "abc".to_owned(),
        };
        let firebase_doc = pod_to_document(&t)?;
        let map = firebase_doc.fields;
        assert_eq!(
            map.unwrap()
                .get("integer_test")
                .expect("a value in the map for integer_test")
                .integer_value
                .as_ref()
                .expect("an integer value"),
            "12"
        );

        Ok(())
    }
}
