use super::*;
use crate::{errors::extract_google_api_error_async, FirebaseAuthBearerAsync};
use async_stream::try_stream;
use futures_core::stream::Stream;
use futures_util::{pin_mut, stream::StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

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
where
    BEARER: FirebaseAuthBearer,
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

/// THIS IS A NON-BLOCKING OPERATION
/// ## Arguments
/// * 'auth' The authentication token
/// * 'collection_id' The document path / collection; For example "my_collection" or "a/nested/collection"
pub fn list_async<T: Clone, BEARER>(auth: BEARER, collection_id: impl Into<String>) -> AsyncList<T, BEARER>
where
    BEARER: FirebaseAuthBearerAsync + Clone,
{
    let collection_id = collection_id.into();
    AsyncList {
        url: firebase_url(auth.project_id(), &collection_id),
        auth: auth.clone(),
        next_page_token: None,
        documents: vec![],
        current: 0,
        done: false,
        collection_id,
        phantom: std::marker::PhantomData,
    }
}

#[inline]
fn get_new_data(collection_id: &str, url: &str, auth: &impl FirebaseAuthBearer) -> Result<dto::ListDocumentsResponse> {
    let resp = auth
        .client()
        .get(url)
        .bearer_auth(auth.access_token().to_owned())
        .send()?;

    let resp = extract_google_api_error(resp, || collection_id.to_owned())?;

    let json: dto::ListDocumentsResponse = resp.json()?;
    Ok(json)
}

#[inline]
fn get_new_data_async(
    collection_id: String,
    url: String,
    mut auth: impl FirebaseAuthBearerAsync,
) -> impl Stream<Item = Result<dto::ListDocumentsResponse>> {
    try_stream! {
        let resp = auth
            .client_async()
            .get(url)
            .bearer_auth(auth.access_token().await.to_string())
            .send()
            .await?;
        let resp = extract_google_api_error_async(resp, || collection_id.to_owned()).await?;

        let json: dto::ListDocumentsResponse = resp.json().await?;
        yield json
    }
}

/// This type is returned as a result by [`list()`].
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

/// This type is returned as a result by [`list_async()`].
/// Use it as an iterator. The paging API is used internally and new pages are fetched lazily.
///
/// Please note that this API acts as an iterator of same-like documents.
/// This type is not suitable if you want to list documents of different types.
#[derive(Clone, Debug)]
pub struct AsyncList<T: Clone, BEARER> {
    auth: BEARER,
    next_page_token: Option<String>,
    documents: Vec<dto::Document>,
    current: usize,
    done: bool,
    url: String,
    collection_id: String,
    phantom: std::marker::PhantomData<T>,
}

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

        let result = document_to_pod(doc);
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

impl<T, BEARER> Stream for AsyncList<T, BEARER>
where
    for<'b> T: Deserialize<'b> + Clone,
    BEARER: FirebaseAuthBearerAsync + Clone,
{
    type Item = Result<(T, dto::Document)>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut s = (*self).clone();
        if s.done {
            return Poll::Ready(None);
        }

        if s.documents.len() <= s.current {
            let url = match &s.next_page_token {
                Some(next_page_token) => format!("{}pageToken={}", s.url, next_page_token),
                None => s.url.clone(),
            };

            let result = get_new_data_async(s.collection_id.clone(), url.clone(), s.auth.clone());
            pin_mut!(result);
            let res = result.poll_next_unpin(cx);
            match res {
                Poll::Ready(item) => match item {
                    None => {
                        return Poll::Ready(None);
                    }
                    Some(result) => match result {
                        Ok(v) => match v.documents {
                            None => return Poll::Ready(None),
                            Some(documents) => {
                                s.documents = documents;
                                s.current = 0;
                                s.next_page_token = v.next_page_token;
                            }
                        },
                        Err(e) => {
                            s.done = true;
                            return Poll::Ready(Some(Err(e)));
                        }
                    },
                },
                Poll::Pending => return Poll::Pending,
            }
        }

        let doc = s.documents.get(s.current).unwrap();

        s.current += 1;
        if s.documents.len() <= s.current && s.next_page_token.is_none() {
            s.done = true;
        }

        let result = document_to_pod(doc);
        match result {
            Err(e) => Poll::Ready(Some(Err(e))),
            Ok(pod) => Poll::Ready(Some(Ok((
                pod,
                dto::Document {
                    update_time: doc.update_time.clone(),
                    create_time: doc.create_time.clone(),
                    name: doc.name.clone(),
                    fields: None,
                },
            )))),
        }
    }
}
