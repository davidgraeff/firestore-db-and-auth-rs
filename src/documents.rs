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

use super::errors::{FirebaseError, Result};


use super::dto;

use super::FirebaseAuthBearer;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use reqwest::Client;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct Wrapper {
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

use serde_json::{map::Map, Number};

fn firebase_value_to_serde_value(v: &dto::Value) -> serde_json::Value {
    if let Some(timestamp_value) = v.timestamp_value.as_ref() {
        return Value::String(timestamp_value.clone());
    } else if let Some(integer_value) = v.integer_value.as_ref() {
        if let Ok(four) = integer_value.parse::<u32>() {
            return Value::Number(four.into());
        }
    } else if let Some(double_value) = v.double_value {
        if let Some(dd) = Number::from_f64(double_value) {
            return Value::Number(dd);
        }
    } else if let Some(map_value) = v.map_value.as_ref() {
        let mut map: Map<String, serde_json::value::Value> = Map::new();
        for (map_key, map_v) in &map_value.fields {
            map.insert(map_key.clone(), firebase_value_to_serde_value(&map_v));
        }
        return Value::Object(map);
    } else if let Some(string_value) = v.string_value.as_ref() {
        return Value::String(string_value.clone());
    } else if let Some(boolean_value) = v.boolean_value {
        return Value::Bool(boolean_value);
    } else if let Some(array_value) = v.array_value.as_ref() {
        let mut vec: Vec<Value> = Vec::new();
        for k in &array_value.values {
            vec.push(firebase_value_to_serde_value(&k));
        }
        return Value::Array(vec);
    }
    return Value::Null;
}

fn serde_value_to_firebase_value(v: &serde_json::Value) -> dto::Value {
    if v.is_f64() {
        return dto::Value {
            double_value: Some(v.as_f64().unwrap()),
            ..Default::default()
        };
    } else if let Some(integer_value) = v.as_i64() {
        return dto::Value {
            integer_value: Some(integer_value.to_string()),
            ..Default::default()
        };
    } else if let Some(map_value) = v.as_object() {
        let mut map: HashMap<String, dto::Value> = HashMap::new();
        for (map_key, map_v) in map_value {
            map.insert(map_key.to_owned(), serde_value_to_firebase_value(&map_v));
        }
        return dto::Value {
            map_value: Some(dto::MapValue { fields: map }),
            ..Default::default()
        };
    } else if let Some(string_value) = v.as_str() {
        return dto::Value {
            string_value: Some(string_value.to_owned()),
            ..Default::default()
        };
    } else if let Some(boolean_value) = v.as_bool() {
        return dto::Value {
            boolean_value: Some(boolean_value),
            ..Default::default()
        };
    } else if let Some(array_value) = v.as_array() {
        let mut vec: Vec<dto::Value> = Vec::new();
        for k in array_value {
            vec.push(serde_value_to_firebase_value(&k));
        }
        return dto::Value {
            array_value: Some(dto::ArrayValue { values: vec }),
            ..Default::default()
        };
    }
    return Default::default();
}

fn document_to_pod<T>(document: &dto::Document) -> Result<T>
where
    for<'de> T: Deserialize<'de>,
{
    let r = Wrapper {
        extra: document
            .fields
            .as_ref()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                return (k.to_owned(), firebase_value_to_serde_value(&v));
            })
            .collect(),
    };

    let v = serde_json::to_value(r)?;
    let r: T = serde_json::from_value(v)?;
    Ok(r)
}

fn pod_to_document<T>(pod: &T) -> Result<dto::Document>
where
    T: Serialize,
{
    let v = serde_json::to_value(pod)?;
    Ok(dto::Document {
        fields: Some(serde_value_to_firebase_value(&v).map_value.unwrap().fields),
        ..Default::default()
    })
}

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

use std::path::Path;

///
/// Write a document to a given collection.
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The document path / collection; For example "my_collection" or "a/nested/collection"
/// * 'document_id' The document id. Make sure that you do not include the document id to the path argument.
/// * 'document' The document
pub fn write<'a, T, BEARER>(
    auth: &'a mut BEARER,
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

    if resp.status() != 200 {
        return Err(FirebaseError::UnexpectedResponse(
            "Firestore write failed: ",
            resp.status(),
            resp.text()?,
            serde_json::to_string_pretty(&firebase_document)?,
        ));
    }
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
pub fn read_by_name<'a, T, BEARER>(auth: &'a mut BEARER, document_name: &str) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
     let url = format!(
        firebase_url_base!(),
        document_name
    );

     let mut resp = Client::new()
        .get(&url)
        .bearer_auth(auth.bearer().to_owned())
        .send()?;

    if resp.status() != 200 {
        return Err(FirebaseError::UnexpectedResponse(
            "Firestore read failed: ",
            resp.status(),
            resp.text()?,
            serde_json::to_string_pretty(&url)?,
        ));
    }

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
pub fn read<'a, T, BEARER>(auth: &'a mut BEARER, path: &str, document_id: &str) -> Result<T>
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
    auth: &'a mut BEARER,
    next_page_token: Option<String>,
    documents: Vec<dto::Document>,
    current: usize,
    done: bool,
    url: String,
    phantom: std::marker::PhantomData<T>,
}

/// List all documents of a given collection.
///
/// Please note that this API acts as an iterator of same-like documents.
/// This type is not suitable if you want to list documents of different types.
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The document path / collection; For example "my_collection" or "a/nested/collection"
pub fn list<'a, T, BEARER>(auth: &'a mut BEARER, path: &str) -> List<'a, T, BEARER> 
where for<'c> BEARER: FirebaseAuthBearer<'c> {
    List {
        url: format!(firebase_url!(), auth.projectid(), path,),
        auth,
        next_page_token: None,
        documents: vec![],
        current: 0,
        done: false,
        phantom: std::marker::PhantomData,
    }
}

fn get_new_data<'a>(
    url: &str,
    auth: &'a mut dyn FirebaseAuthBearer<'a>,
) -> Result<dto::ListDocumentsResponse> {
    let mut resp = Client::new()
        .get(url)
        .bearer_auth(auth.bearer().to_owned())
        .send()?;

    if resp.status() != 200 {
        return Err(FirebaseError::UnexpectedResponse(
            "Firestore read failed: ",
            resp.status(),
            resp.text()?,
            serde_json::to_string_pretty(&url)?,
        ));
    }

    let json: dto::ListDocumentsResponse = resp.json()?;
    Ok(json)
}

impl<'a, T, BEARER> Iterator for List<'a, T, BEARER>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        if self.documents.len() <= self.current {

            let url = match &self.next_page_token {
                Some(next_page_token) => format!("{}pageToken={}", self.url, next_page_token),
                None => self.url.clone(),
            };

            let result = get_new_data(&url, self.auth);
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

        return Some(document_to_pod(&doc));
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
    auth: &'a mut BEARER,
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

 if resp.status() != 200 {
        return Err(FirebaseError::UnexpectedResponse(
            "Firestore query failed: ",
            resp.status(),
            resp.text()?,
            serde_json::to_string_pretty(&url)?,
        ));
    }

    let json: Option<Vec<dto::RunQueryResponse>> = resp.json()?;

    let mut dtos: Vec<T> = Vec::new();
    if json.is_none() {
        return Ok(dtos);
    }
    let json = json.unwrap();

    for value in json.iter() {
        if let Some(ref document) = &value.document {
            if document.fields.is_none() && document.name.is_some() {
                let doc : T = read_by_name(auth, &document.name.as_ref().unwrap())?;
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
pub fn delete<'a, BEARER>(auth: &'a mut BEARER, path: &str) -> Result<()>
where
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let url = format!(firebase_url!(), auth.projectid(), path);

    Client::new()
        .delete(&url)
        .bearer_auth(auth.bearer().to_owned())
        .send()?;

    Ok({})
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::Result;
    use serde::{Deserialize, Serialize};

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
            map.unwrap().get("integer_test")
                .expect("a value in the map for integer_test")
                .integer_value
                .as_ref()
                .expect("an integer value"),
            "12"
        );

        Ok(())
    }
}