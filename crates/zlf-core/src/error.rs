use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZlfError {
    #[error("Node with ID '{0}' already exists")]
    NodeAlreadyExists(String),

    #[error("Node with ID '{0}' not found")]
    NodeNotFound(String),

    #[error("Edge with ID '{0}' already exists")]
    EdgeAlreadyExists(String),

    #[error("Edge with ID '{0}' not found")]
    EdgeNotFound(String),

    #[error("Source node '{0}' not found")]
    SourceNodeNotFound(String),

    #[error("Target node '{0}' not found")]
    TargetNodeNotFound(String),

    #[error("Edge type cannot be empty")]
    EmptyEdgeType,

    #[error("Node ID too long (max 255 chars)")]
    NodeIdTooLong,

    #[error("Invalid property value for key '{0}'")]
    InvalidPropertyValue(String),

    #[error("Version conflict for node '{0}'")]
    VersionConflict(String),

    #[error("Max versions exceeded for node '{0}' (max 1000)")]
    MaxVersionsExceeded(String),

    #[error("Storage quota exceeded")]
    StorageQuotaExceeded,

    #[error("Invalid memory type: {0}")]
    InvalidMemoryType(String),

    #[error("Predicate '{0}' not defined")]
    PredicateNotDefined(String),

    #[error("Query timeout after {0}ms")]
    QueryTimeout(u64),

    #[error("Results truncated to {0}")]
    ResultsTruncated(usize),

    #[error("Syntax error at line {0}: {1}")]
    SyntaxError(usize, String),

    #[error("Feature not supported: {0}")]
    UnsupportedFeature(String),

    #[error("Embedding dimension mismatch: expected {0}, got {1}")]
    EmbeddingDimensionMismatch(usize, usize),

    #[error("Embedding service unavailable: {0}")]
    EmbeddingServiceUnavailable(String),

    #[error("Embedding timeout after {0}ms")]
    EmbeddingTimeout(u64),

    #[error("Invalid embedding values")]
    InvalidEmbeddingValues,

    #[error("Node '{0}' has no embedding")]
    NoEmbedding(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Database already exists: {0}")]
    DatabaseAlreadyExists(String),

    #[error("Insufficient disk space")]
    InsufficientDiskSpace,

    #[error("Backup integrity check failed")]
    BackupIntegrityFailed,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ZlfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = ZlfError::NodeNotFound("alice".to_string());
        assert_eq!(err.to_string(), "Node with ID 'alice' not found");

        let err = ZlfError::EmptyEdgeType;
        assert_eq!(err.to_string(), "Edge type cannot be empty");

        let err = ZlfError::QueryTimeout(30000);
        assert_eq!(err.to_string(), "Query timeout after 30000ms");
    }
}
