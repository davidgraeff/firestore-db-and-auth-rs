use super::*;

/// List all documents of a given collection.
///
/// Please note that this API acts as an iterator of same-like documents.
/// This type is not suitable if you want to list documents of different types.
///
/// Example:
/// ```no_run
/// # use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize)]
/// struct DemoDTO { a_string: String, an_int: u32, }
///
/// use firestore_db_and_auth::documents;
/// # use firestore_db_and_auth::{credentials::Credentials, ServiceSession, errors::Result};
/// # use firestore_db_and_auth::credentials::doctest_credentials;
/// # let session = ServiceSession::new(doctest_credentials())?;
///
/// let values: documents::List<DemoDTO, _> = documents::list(&session, "tests");
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
pub fn list<T, BEARER>(auth: &BEARER, collection_id: impl Into<String>) -> List<T, BEARER>
    where BEARER: FirebaseAuthBearer,
{
    let collection_id = collection_id.into();
    List {
        url: firebase_url(auth.project_id(), &collection_id),
        auth,
        next_page_token: None,
        documents: vec![],
        current: 0,
        done: false,
        collection_id,
        phantom: std::marker::PhantomData,
    }
}

#[inline]
async fn get_new_data<'a>(
    collection_id: &str,
    url: &str,
    auth: &'a impl FirebaseAuthBearer,
) -> Result<dto::ListDocumentsResponse> {
    let resp = auth
        .client_async()
        .get(url)
        .bearer_auth(auth.access_token().to_owned())
        .send().await?;

    let resp = extract_google_api_error_async(resp, || collection_id.to_owned()).await?;

    let json: dto::ListDocumentsResponse = resp.json().await?;
    Ok(json)
}

/// This type is returned as a result by [`list`].
/// Use it as an iterator. The paging API is used internally and new pages are fetched lazily.
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

#[cfg(feature = "blocking")]
impl<'a, T, BEARER> Iterator for List<'a, T, BEARER>
    where
            for<'b> T: Deserialize<'b>,
            BEARER: FirebaseAuthBearer,
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

            let result = self.auth.rt().block_on(get_new_data(&self.collection_id, &url, self.auth));
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
