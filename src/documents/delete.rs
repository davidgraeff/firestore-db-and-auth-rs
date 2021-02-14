use super::*;

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
pub async fn delete(auth: &impl FirebaseAuthBearer, path: &str, fail_if_not_existing: bool) -> Result<()> {
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
        .client_async()
        .delete(&url)
        .bearer_auth(auth.access_token().to_owned())
        .json(&query_request)
        .send()
        .await?;

    extract_google_api_error_async(resp, || path.to_owned()).await?;

    Ok({})
}

#[cfg(feature = "blocking")]
pub mod blocking {
    use super::*;

    /// Deletes the document at the given path.
    /// See [`super::delete()`]
    pub fn delete(auth: &impl FirebaseAuthBearer, path: &str, fail_if_not_existing: bool) -> Result<()> {
        auth.rt().block_on(super::delete(auth, path, fail_if_not_existing))?;
        Ok(())
    }
}