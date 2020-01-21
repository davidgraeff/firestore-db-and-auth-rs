use super::*;

///
/// Read a document of a specific type from a collection by its Firestore document name
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'document_name' The document path / collection and document id; For example "projects/my_project/databases/(default)/documents/tests/test"
pub fn read_by_name<T>(auth: &impl FirebaseAuthBearer, document_name: impl AsRef<str>) -> Result<T>
    where
            for<'b> T: Deserialize<'b>,
{
    let url = firebase_url_base(document_name.as_ref());

    let resp = auth
        .client()
        .get(&url)
        .bearer_auth(auth.access_token().to_owned())
        .send()?;


    let json: dto::Document = extract_google_api_response(resp, || document_name.as_ref().to_owned())?;
    Ok(document_to_pod(&json)?)
}

///
/// Read a document of a specific type from a collection
///
/// ## Arguments
/// * 'auth' The authentication token
/// * 'path' The document path / collection; For example "my_collection" or "a/nested/collection"
/// * 'document_id' The document id. Make sure that you do not include the document id to the path argument.
pub fn read<T>(auth: &impl FirebaseAuthBearer, path: &str, document_id: impl AsRef<str>) -> Result<T>
    where
            for<'b> T: Deserialize<'b>,
{
    let document_name = format!(
        "projects/{}/databases/(default)/documents/{}/{}",
        auth.project_id(),
        path,
        document_id.as_ref()
    );
    read_by_name(auth, &document_name)
}
