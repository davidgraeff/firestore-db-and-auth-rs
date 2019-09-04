use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GoogleFirestoreAdminv1IndexField {
    #[serde(rename = "fieldPath")]
    pub field_path: Option<String>,
    pub mode: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ListenResponse {
    pub filter: Option<ExistenceFilter>,
    #[serde(rename = "targetChange")]
    pub target_change: Option<TargetChange>,
    #[serde(rename = "documentDelete")]
    pub document_delete: Option<DocumentDelete>,
    #[serde(rename = "documentChange")]
    pub document_change: Option<DocumentChange>,
    #[serde(rename = "documentRemove")]
    pub document_remove: Option<DocumentRemove>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct BeginTransactionResponse {
    pub transaction: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Write {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "currentDocument")]
    pub current_document: Option<Precondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<Document>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<DocumentTransform>,
    #[serde(rename = "updateMask")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_mask: Option<DocumentMask>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum FieldOperator {
    OPERATOR_UNSPECIFIED,  //	Unspecified. This value must not be used.
    LESS_THAN,             //	Less than. Requires that the field come first in orderBy.
    LESS_THAN_OR_EQUAL,    //	Less than or equal. Requires that the field come first in orderBy.
    GREATER_THAN,          //	Greater than. Requires that the field come first in orderBy.
    GREATER_THAN_OR_EQUAL, //	Greater than or equal. Requires that the field come first in orderBy.
    EQUAL,                 //	Equal.
    ARRAY_CONTAINS,        //	Contains. Requires that the field is an array.
}

impl Default for FieldOperator {
    fn default() -> Self {
        FieldOperator::OPERATOR_UNSPECIFIED
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct FieldFilter {
    pub field: FieldReference,
    pub value: Value,
    pub op: FieldOperator,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GoogleFirestoreAdminv1ImportDocumentsRequest {
    #[serde(rename = "inputUriPrefix")]
    pub input_uri_prefix: Option<String>,
    #[serde(rename = "collectionIds")]
    pub collection_ids: Option<Vec<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub fields: Option<HashMap<String, Value>>,
    #[serde(rename = "updateTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(rename = "createTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GoogleFirestoreAdminv1ListIndexesResponse {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    pub indexes: Option<Vec<GoogleFirestoreAdminv1Index>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct BatchGetDocumentsResponse {
    pub found: Option<Document>,
    pub transaction: Option<String>,
    #[serde(rename = "readTime")]
    pub read_time: Option<String>,
    pub missing: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub message: Option<String>,
    pub code: Option<i32>,
    pub details: Option<Vec<HashMap<String, String>>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ListenRequest {
    pub labels: Option<HashMap<String, String>>,
    #[serde(rename = "addTarget")]
    pub add_target: Option<Target>,
    #[serde(rename = "removeTarget")]
    pub remove_target: Option<i32>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RunQueryRequest {
    #[serde(rename = "newTransaction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_transaction: Option<TransactionOptions>,
    pub transaction: Option<String>,
    #[serde(rename = "structuredQuery")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_query: Option<StructuredQuery>,
    #[serde(rename = "readTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct FieldReference {
    #[serde(rename = "fieldPath")]
    pub field_path: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct UnaryFilter {
    pub field: FieldReference,
    pub op: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ArrayValue {
    pub values: Option<Vec<Value>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMask {
    #[serde(rename = "fieldPaths")]
    pub field_paths: Vec<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CompositeFilter {
    pub filters: Vec<Filter>,
    pub op: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Empty {
    _never_set: Option<bool>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Filter {
    #[serde(rename = "unaryFilter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unary_filter: Option<UnaryFilter>,
    #[serde(rename = "fieldFilter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_filter: Option<FieldFilter>,
    #[serde(rename = "compositeFilter")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_filter: Option<CompositeFilter>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct WriteResponse {
    #[serde(rename = "writeResults")]
    pub write_results: Option<Vec<WriteResult>>,
    #[serde(rename = "streamToken")]
    pub stream_token: Option<String>,
    #[serde(rename = "commitTime")]
    pub commit_time: Option<String>,
    #[serde(rename = "streamId")]
    pub stream_id: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ListCollectionIdsRequest {
    #[serde(rename = "pageToken")]
    pub page_token: Option<String>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i32>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct BatchGetDocumentsRequest {
    #[serde(rename = "newTransaction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_transaction: Option<TransactionOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<DocumentMask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<String>>,
    #[serde(rename = "readTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MapValue {
    pub fields: HashMap<String, Value>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct TransactionOptions {
    #[serde(rename = "readWrite")]
    pub read_write: Option<ReadWrite>,
    #[serde(rename = "readOnly")]
    pub read_only: Option<ReadOnly>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CommitResponse {
    #[serde(rename = "writeResults")]
    pub write_results: Option<Vec<WriteResult>>,
    #[serde(rename = "commitTime")]
    pub commit_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Target {
    pub documents: Option<DocumentsTarget>,
    pub once: Option<bool>,
    pub query: Option<QueryTarget>,
    #[serde(rename = "resumeToken")]
    pub resume_token: Option<String>,
    #[serde(rename = "targetId")]
    pub target_id: Option<i32>,
    #[serde(rename = "readTime")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ExistenceFilter {
    pub count: Option<i32>,
    #[serde(rename = "targetId")]
    pub target_id: Option<i32>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct DocumentsTarget {
    pub documents: Option<Vec<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Precondition {
    #[serde(rename = "updateTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Value {
    #[serde(rename = "bytesValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_value: Option<String>,

    #[serde(rename = "timestampValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_value: Option<String>,

    #[serde(rename = "geoPointValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo_point_value: Option<LatLng>,

    #[serde(rename = "referenceValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_value: Option<String>,

    #[serde(rename = "doubleValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double_value: Option<f64>,

    #[serde(rename = "mapValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_value: Option<MapValue>,

    #[serde(rename = "stringValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,

    #[serde(rename = "booleanValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boolean_value: Option<bool>,

    #[serde(rename = "arrayValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_value: Option<ArrayValue>,

    #[serde(rename = "integerValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integer_value: Option<String>,

    #[serde(rename = "nullValue")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_value: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Cursor {
    pub values: Option<Vec<Value>>,
    pub before: Option<bool>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CollectionSelector {
    #[serde(rename = "allDescendants")]
    pub all_descendants: Option<bool>,
    #[serde(rename = "collectionId")]
    pub collection_id: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GoogleFirestoreAdminv1Index {
    pub fields: Option<Vec<GoogleFirestoreAdminv1IndexField>>,
    pub state: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "collectionId")]
    pub collection_id: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct StructuredQuery {
    #[serde(rename = "orderBy")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<Order>>,
    #[serde(rename = "startAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<Cursor>,
    #[serde(rename = "endAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at: Option<Cursor>,
    pub limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Vec<CollectionSelector>>,
    #[serde(rename = "where")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Projection>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct FieldTransform {
    #[serde(rename = "fieldPath")]
    pub field_path: Option<String>,
    #[serde(rename = "appendMissingElements")]
    pub append_missing_elements: Option<ArrayValue>,
    #[serde(rename = "setToServerValue")]
    pub set_to_server_value: Option<String>,
    #[serde(rename = "removeAllFromArray")]
    pub remove_all_from_array: Option<ArrayValue>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct DocumentDelete {
    #[serde(rename = "removedTargetIds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_target_ids: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    #[serde(rename = "readTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GoogleFirestoreAdminv1ExportDocumentsRequest {
    #[serde(rename = "outputUriPrefix")]
    pub output_uri_prefix: Option<String>,
    #[serde(rename = "collectionIds")]
    pub collection_ids: Option<Vec<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<FieldReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct TargetChange {
    #[serde(rename = "resumeToken")]
    pub resume_token: Option<String>,
    #[serde(rename = "targetChangeType")]
    pub target_change_type: Option<String>,
    pub cause: Option<Status>,
    #[serde(rename = "targetIds")]
    pub target_ids: Option<Vec<i32>>,
    #[serde(rename = "readTime")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RunQueryResponse {
    #[serde(rename = "skippedResults")]
    pub skipped_results: Option<i32>,
    pub transaction: Option<String>,
    pub document: Option<Document>,
    #[serde(rename = "readTime")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ListCollectionIdsResponse {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "collectionIds")]
    pub collection_ids: Option<Vec<String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CommitRequest {
    pub writes: Option<Vec<Write>>,
    pub transaction: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Projection {
    pub fields: Option<Vec<FieldReference>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ListDocumentsResponse {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    pub documents: Option<Vec<Document>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ReadWrite {
    #[serde(rename = "retryTransaction")]
    pub retry_transaction: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct GoogleLongrunningOperation {
    pub error: Option<Status>,
    pub done: Option<bool>,
    pub response: Option<HashMap<String, String>>,
    pub name: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct LatLng {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct DocumentChange {
    #[serde(rename = "removedTargetIds")]
    pub removed_target_ids: Option<Vec<i32>>,
    pub document: Option<Document>,
    #[serde(rename = "targetIds")]
    pub target_ids: Option<Vec<i32>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct DocumentRemove {
    #[serde(rename = "removedTargetIds")]
    pub removed_target_ids: Option<Vec<i32>>,
    pub document: Option<String>,
    #[serde(rename = "readTime")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub transaction: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ReadOnly {
    #[serde(rename = "readTime")]
    pub read_time: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct BeginTransactionRequest {
    pub options: Option<TransactionOptions>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct DocumentTransform {
    pub document: Option<String>,
    #[serde(rename = "fieldTransforms")]
    pub field_transforms: Option<Vec<FieldTransform>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct WriteResult {
    #[serde(rename = "updateTime")]
    pub update_time: Option<String>,
    #[serde(rename = "transformResults")]
    pub transform_results: Option<Vec<Value>>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct QueryTarget {
    #[serde(rename = "structuredQuery")]
    pub structured_query: Option<StructuredQuery>,
    pub parent: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct WriteRequest {
    pub writes: Option<Vec<Write>>,
    pub labels: Option<HashMap<String, String>>,
    #[serde(rename = "streamToken")]
    pub stream_token: Option<String>,
    #[serde(rename = "streamId")]
    pub stream_id: Option<String>,
}
