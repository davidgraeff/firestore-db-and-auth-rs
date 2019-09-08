use super::*;

///
/// Read a document of a specific type from a collection by its Firestore document name
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'document_name' The document path / collection and document id; For example "projects/my_project/databases/(default)/documents/tests/test"
pub fn read_by_name<'a, T, BEARER>(auth: &'a BEARER, document_name: impl AsRef<str>) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
    BEARER: FirebaseAuthBearer<'a>,
{
    let url = firebase_url_base(document_name.as_ref());

    let mut resp = auth
        .client()
        .get(&url)
        .bearer_auth(auth.access_token().to_owned())
        .send()?;

    extract_google_api_error(&mut resp, || document_name.as_ref().to_owned())?;

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
pub fn read<'a, T, BEARER>(auth: &'a BEARER, path: &str, document_id: impl AsRef<str>) -> Result<T>
where
    for<'b> T: Deserialize<'b>,
    for<'c> BEARER: FirebaseAuthBearer<'c>,
{
    let document_name = format!(
        "projects/{}/databases/(default)/documents/{}/{}",
        auth.project_id(),
        path,
        document_id.as_ref()
    );
    read_by_name(auth, &document_name)
}
