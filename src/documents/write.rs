use super::*;

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

/// Write options. The default will overwrite a target document and not merge fields.
#[derive(Default)]
pub struct WriteOptions {
    /// If this is set instead of overwriting all fields of a target document, only the given fields will be merged.
    /// This only works if your document type has Option fields.
    /// The write will fail, if no document_id is given or the target document does not exist yet.
    pub merge: bool,
}

///
/// Write a document to a given collection.
///
/// If no document_id is given, Firestore will generate an ID. Check the [`WriteResult`] return value.
///
/// If a document_id is given, the document will be created if it does not yet exist.
/// Except if the "merge" option (see [`WriteOptions::merge`]) is set.
///
/// Example:
///```no_run
///use firestore_db_and_auth::{Credentials, ServiceSession, documents, errors::Result, FirebaseAuthBearer};
///use serde::{Serialize,Deserialize};
///# use firestore_db_and_auth::credentials::doctest_credentials;
///
/// #[derive(Serialize, Deserialize)]
/// struct DemoDTO {
///    a_string: String,
///    an_int: u32,
///    another_int: u32,
/// }
/// #[derive(Serialize, Deserialize)]
/// struct DemoPartialDTO {
///    #[serde(skip_serializing_if = "Option::is_none")]
///    a_string: Option<String>,
///    an_int: u32,
/// }
///
/// async fn write(session: &impl FirebaseAuthBearer) -> Result<()> {
///    let obj = DemoDTO { a_string: "abcd".to_owned(), an_int: 14, another_int: 10 };
///    let result = documents::write(session, "tests", Some("service_test"), &obj, documents::WriteOptions::default()).await?;
///    println!("id: {}, created: {}, updated: {}", result.document_id, result.create_time.unwrap(), result.update_time.unwrap());
///    Ok(())
/// }
/// /// Only write some fields and do not overwrite the entire document.
/// /// Either via Option<> or by not having the fields in the structure, see DemoPartialDTO.
/// async fn write_partial(session: &impl FirebaseAuthBearer) -> Result<()> {
///    let obj = DemoPartialDTO { a_string: None, an_int: 16 };
///    let result = documents::write(session, "tests", Some("service_test"), &obj, documents::WriteOptions{merge:true}).await?;
///    println!("id: {}, created: {}, updated: {}", result.document_id, result.create_time.unwrap(), result.update_time.unwrap());
///    Ok(())
/// }
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// #   let session = ServiceSession::new(doctest_credentials().await).await?;
/// #   write(&session).await?;
/// #   write_partial(&session).await?;
/// #
/// #   Ok(())
/// # }
///```

///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The document path / collection; For example "my_collection" or "a/nested/collection"
/// * 'document_id' The document id. Make sure that you do not include the document id in the path argument.
/// * 'document' The document
/// * 'options' Write options
pub async fn write<T>(
    auth: &impl FirebaseAuthBearer,
    path: &str,
    document_id: Option<impl AsRef<str>>,
    document: &T,
    options: WriteOptions,
) -> Result<WriteResult>
where
    T: Serialize,
{
    let mut url = match document_id.as_ref() {
        Some(document_id) => firebase_url_extended(auth.project_id(), path, document_id.as_ref()),
        None => firebase_url(auth.project_id(), path),
    };

    let firebase_document = pod_to_document(&document)?;

    if options.merge && firebase_document.fields.is_some() {
        let fields = firebase_document.fields.as_ref().unwrap().keys().join(",");
        url = format!("{}?currentDocument.exists=true&updateMask.fieldPaths={}", url, fields);
    }

    let builder = if document_id.is_some() {
        auth.client().patch(&url)
    } else {
        auth.client().post(&url)
    };

    let resp = builder
        .bearer_auth(auth.access_token().await.to_owned())
        .json(&firebase_document)
        .send()
        .await?;

    let resp = extract_google_api_error_async(resp, || {
        document_id
            .as_ref()
            .and_then(|f| Some(f.as_ref().to_owned()))
            .or(Some(String::new()))
            .unwrap()
    })
    .await?;

    let result_document: dto::Document = resp.json().await?;
    let document_id = Path::new(&result_document.name)
        .file_name()
        .ok_or_else(|| FirebaseError::Generic("Resulting documents 'name' field is not a valid path"))?
        .to_str()
        .ok_or_else(|| FirebaseError::Generic("No valid unicode in 'name' field"))?
        .to_owned();

    let create_time = match result_document.create_time {
        Some(f) => Some(
            chrono::DateTime::parse_from_rfc3339(&f)
                .map_err(|_| FirebaseError::Generic("Failed to parse rfc3339 date from 'create_time' field"))?
                .with_timezone(&chrono::Utc),
        ),
        None => None,
    };
    let update_time = match result_document.update_time {
        Some(f) => Some(
            chrono::DateTime::parse_from_rfc3339(&f)
                .map_err(|_| FirebaseError::Generic("Failed to parse rfc3339 date from 'update_time' field"))?
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
