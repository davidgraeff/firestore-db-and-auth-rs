use super::*;
use std::vec::IntoIter;

///
/// Queries the database for specific documents, for example all documents in a collection of 'type' == "car".
///
/// Example:
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize)]
/// struct DemoDTO { a_string: String, an_int: u32, }
///
/// use firestore_db_and_auth::{documents, dto};
/// # use firestore_db_and_auth::{credentials::Credentials, ServiceSession, errors::Result};
///
/// # let credentials = Credentials::new(include_str!("../../firebase-service-account.json"),
///                                         &[include_str!("../../tests/service-account-for-tests.jwks")])?;
/// # let session = ServiceSession::new(credentials)?;
///
/// let values: documents::Query = documents::query(&session, "tests", "Sam Weiss".into(), dto::FieldOperator::EQUAL, "id")?;
/// for metadata in values {
///     println!("id: {}, created: {}, updated: {}", metadata.name.as_ref().unwrap(), metadata.create_time.as_ref().unwrap(), metadata.update_time.as_ref().unwrap());
///     // Fetch the actual document
///     // The data is wrapped in a Result<> because fetching new data could have failed
///     let doc : DemoDTO = documents::read_by_name(&session, metadata.name.as_ref().unwrap())?;
///     println!("{:?}", doc);
/// }
/// # Ok::<(), firestore_db_and_auth::errors::FirebaseError>(())
/// ```
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'collectionid' The collection id; "my_collection" or "a/nested/collection"
/// * 'value' The query / filter value. For example "car".
/// * 'operator' The query operator. For example "EQUAL".
/// * 'field' The query / filter field. For example "type".
pub fn query<'a, BEARER>(
    auth: &'a BEARER,
    collection_id: &str,
    value: serde_json::Value,
    operator: dto::FieldOperator,
    field: &str,
) -> Result<Query>
    where
            for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let url = firebase_url_query(auth.project_id());
    let value = crate::firebase_rest_to_rust::serde_value_to_firebase_value(&value);

    let query_request = dto::RunQueryRequest {
        structured_query: Some(dto::StructuredQuery {
            select: Some(dto::Projection { fields: None }),
            where_: Some(dto::Filter {
                field_filter: Some(dto::FieldFilter {
                    value,
                    op: operator,
                    field: dto::FieldReference {
                        field_path: field.to_owned(),
                    },
                }),
                ..Default::default()
            }),
            from: Some(vec![dto::CollectionSelector {
                collection_id: Some(collection_id.to_owned()),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut resp = auth.client()
        .post(&url)
        .bearer_auth(auth.access_token().to_owned())
        .json(&query_request)
        .send()?;

    extract_google_api_error(&mut resp, || collection_id.to_owned())?;

    let json: Option<Vec<dto::RunQueryResponse>> = resp.json()?;

    Ok(Query(json.unwrap_or_default().into_iter()))
}


/// This type is returned as a result by [`query`].
/// Use it as an iterator. The query API returns a list of document references, not the documents itself.
///
/// If you just need the meta data like the document name or update time, you are already settled.
/// To fetch the document itself, use [`read_by_name`].
///
/// Please note that this API acts as an iterator of same-like documents.
/// This type is not suitable if you want to list documents of different types.
pub struct Query(IntoIter<dto::RunQueryResponse>);

impl Iterator for Query {
    type Item = dto::Document;

    // Skip empty entries
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(r) = self.0.next() {
            if let Some(document) = r.document {
                return Some(document);
            }
        }
        return None;
    }
}
