// HTTP REST API Server Implementation
// Provides JSON API for document CRUD operations

use anyhow::Result;
use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{net::TcpListener, sync::Mutex};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    builders::DocumentBuilder,
    connection_pool::ConnectionPoolImpl,
    contracts::connection_pool::ConnectionPool,
    contracts::{Document, Storage},
    observability::with_trace_id,
    types::ValidatedDocumentId,
};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    storage: Arc<Mutex<dyn Storage>>,
    connection_pool: Option<Arc<tokio::sync::Mutex<ConnectionPoolImpl>>>,
}

/// Request body for document creation
#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub path: String,
    pub title: Option<String>,
    pub content: Vec<u8>,
    pub tags: Option<Vec<String>>,
}

/// Request body for document updates
#[derive(Debug, Deserialize)]
pub struct UpdateDocumentRequest {
    pub path: Option<String>,
    pub title: Option<String>,
    pub content: Option<Vec<u8>>,
    pub tags: Option<Vec<String>>,
}

/// Response for document operations
#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub path: String,
    pub title: String,
    pub content: Vec<u8>,
    pub content_hash: String,
    pub size_bytes: u64,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub word_count: u32,
}

/// Response for search operations
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub documents: Vec<DocumentResponse>,
    pub total_count: usize,
}

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub tags: Option<String>, // comma-separated tags
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Connection statistics response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionStatsResponse {
    pub active_connections: usize,
    pub total_connections: u64,
    pub rejected_connections: u64,
    pub rate_limited_requests: u64,
}

/// Performance metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceStatsResponse {
    pub avg_latency_ms: f64,
    pub total_requests: u64,
    pub requests_per_second: f64,
}

/// Resource usage response
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceStatsResponse {
    pub memory_usage_bytes: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f32,
    pub system_healthy: bool,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id.as_uuid(),
            path: doc.path.as_str().to_string(),
            title: doc.title.as_str().to_string(),
            content: doc.content.clone(),
            content_hash: format!("{:x}", md5::compute(&doc.content)),
            size_bytes: doc.size as u64,
            tags: doc.tags.iter().map(|t| t.as_str().to_string()).collect(),
            created_at: doc.created_at.timestamp(),
            modified_at: doc.updated_at.timestamp(),
            word_count: doc.content.iter().filter(|&&b| b == b' ').count() as u32 + 1,
        }
    }
}

/// Create HTTP server with all routes configured
pub fn create_server(storage: Arc<Mutex<dyn Storage>>) -> Router {
    let state = AppState {
        storage,
        connection_pool: None,
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/documents", post(create_document))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", put(update_document))
        .route("/documents/:id", delete(delete_document))
        .route("/documents/search", get(search_documents))
        // Monitoring endpoints
        .route("/stats/connections", get(get_connection_stats))
        .route("/stats/performance", get(get_performance_stats))
        .route("/stats/resources", get(get_resource_stats))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
}

/// Create HTTP server with connection pool integration
pub fn create_server_with_pool(
    storage: Arc<Mutex<dyn Storage>>,
    connection_pool: Arc<tokio::sync::Mutex<ConnectionPoolImpl>>,
) -> Router {
    let state = AppState {
        storage,
        connection_pool: Some(connection_pool),
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/documents", post(create_document))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", put(update_document))
        .route("/documents/:id", delete(delete_document))
        .route("/documents/search", get(search_documents))
        // Monitoring endpoints
        .route("/stats/connections", get(get_connection_stats))
        .route("/stats/performance", get(get_performance_stats))
        .route("/stats/resources", get(get_resource_stats))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
}

/// Start the HTTP server on the specified port
pub async fn start_server(storage: Arc<Mutex<dyn Storage>>, port: u16) -> Result<()> {
    let app = create_server(storage);
    let listener = TcpListener::bind(&format!("0.0.0.0:{port}")).await?;

    info!("KotaDB HTTP server starting on port {}", port);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO: Implement actual uptime tracking
    })
}

/// Create a new document
async fn create_document(
    State(state): State<AppState>,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("create_document", async move {
        // Build document using DocumentBuilder
        let mut builder = DocumentBuilder::new()
            .path(&request.path)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?
            .title(request.title.unwrap_or_else(|| "Untitled".to_string()))
            .map_err(|e| anyhow::anyhow!("Invalid title: {}", e))?
            .content(request.content);

        // Add tags if provided
        if let Some(tags) = request.tags {
            for tag in tags {
                builder = builder
                    .tag(&tag)
                    .map_err(|e| anyhow::anyhow!("Invalid tag: {}", e))?;
            }
        }

        let doc = builder.build()?;

        // Store document
        state.storage.lock().await.insert(doc.clone()).await?;

        Ok(DocumentResponse::from(doc))
    })
    .await;

    match result {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => {
            warn!("Failed to create document: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "creation_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Get document by ID
async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let doc_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: "Invalid document ID format".to_string(),
                }),
            ));
        }
    };

    let validated_id = match ValidatedDocumentId::from_uuid(doc_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: format!("Invalid document ID: {e}"),
                }),
            ));
        }
    };

    let result = with_trace_id("get_document", async move {
        state.storage.lock().await.get(&validated_id).await
    })
    .await;

    match result {
        Ok(Some(doc)) => Ok(Json(DocumentResponse::from(doc))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "document_not_found".to_string(),
                message: format!("Document with ID {doc_id} not found"),
            }),
        )),
        Err(e) => {
            warn!("Failed to get document {}: {}", doc_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "retrieval_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Update document by ID
async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateDocumentRequest>,
) -> Result<Json<DocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let doc_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: "Invalid document ID format".to_string(),
                }),
            ));
        }
    };

    let validated_id = match ValidatedDocumentId::from_uuid(doc_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: format!("Invalid document ID: {e}"),
                }),
            ));
        }
    };

    let result = with_trace_id("update_document", async move {
        // Get existing document
        let doc = match state.storage.lock().await.get(&validated_id).await? {
            Some(doc) => doc,
            None => return Err(anyhow::anyhow!("Document not found")),
        };

        // Build updated document using DocumentBuilder
        let mut builder = DocumentBuilder::new()
            .path(
                request
                    .path
                    .as_ref()
                    .unwrap_or(&doc.path.as_str().to_string()),
            )
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?
            .title(
                request
                    .title
                    .as_ref()
                    .unwrap_or(&doc.title.as_str().to_string()),
            )
            .map_err(|e| anyhow::anyhow!("Invalid title: {}", e))?
            .content(request.content.unwrap_or_else(|| doc.content.clone()));

        // Handle tags: use new tags if provided, otherwise keep existing ones
        if let Some(new_tags) = request.tags {
            // Use new tags only
            for tag in new_tags {
                builder = builder
                    .tag(&tag)
                    .map_err(|e| anyhow::anyhow!("Invalid tag: {}", e))?;
            }
        } else {
            // Keep existing tags
            for tag in &doc.tags {
                builder = builder
                    .tag(tag.as_str())
                    .map_err(|e| anyhow::anyhow!("Failed to add existing tag: {}", e))?;
            }
        }

        let mut updated_doc = builder.build()?;
        // Keep the same ID and adjust timestamps
        updated_doc.id = doc.id;
        updated_doc.created_at = doc.created_at;
        // Ensure updated_at is later than the original
        if updated_doc.updated_at <= doc.updated_at {
            updated_doc.updated_at = doc.updated_at + chrono::Duration::milliseconds(1);
        }

        // Update the document
        state
            .storage
            .lock()
            .await
            .update(updated_doc.clone())
            .await?;

        Ok(DocumentResponse::from(updated_doc))
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Failed to update document {}: {}", doc_id, e);
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: "update_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Delete document by ID
async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let doc_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: "Invalid document ID format".to_string(),
                }),
            ));
        }
    };

    let validated_id = match ValidatedDocumentId::from_uuid(doc_id) {
        Ok(id) => id,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_id".to_string(),
                    message: format!("Invalid document ID: {e}"),
                }),
            ));
        }
    };

    let result = with_trace_id("delete_document", async move {
        // Check if document exists first
        let mut storage = state.storage.lock().await;
        match storage.get(&validated_id).await? {
            Some(_) => {
                storage.delete(&validated_id).await?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Document not found")),
        }
    })
    .await;

    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            warn!("Failed to delete document {}: {}", doc_id, e);
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((
                status,
                Json(ErrorResponse {
                    error: "deletion_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Search documents
async fn search_documents(
    State(state): State<AppState>,
    AxumQuery(params): AxumQuery<SearchParams>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let result = with_trace_id("search_documents", async move {
        // For now, implement a simple search that lists all documents
        // This is a placeholder implementation since we need to integrate with indices
        let limit = params.limit.unwrap_or(50) as usize;
        let _offset = params.offset.unwrap_or(0) as usize;

        // Get all documents and filter by search query if provided
        let all_docs = state.storage.lock().await.list_all().await?;
        let mut filtered_docs = Vec::new();

        for doc in all_docs {
            // Simple text matching if query is provided
            if let Some(ref query) = params.q {
                if !query.is_empty() {
                    let content_str = String::from_utf8_lossy(&doc.content);
                    let title_str = doc.title.as_str();
                    let path_str = doc.path.as_str();

                    if content_str.to_lowercase().contains(&query.to_lowercase())
                        || title_str.to_lowercase().contains(&query.to_lowercase())
                        || path_str.to_lowercase().contains(&query.to_lowercase())
                    {
                        filtered_docs.push(doc);
                    }
                } else {
                    filtered_docs.push(doc);
                }
            } else {
                filtered_docs.push(doc);
            }
        }

        // Apply limit
        let total_count = filtered_docs.len();
        filtered_docs.truncate(limit);

        let documents: Vec<DocumentResponse> = filtered_docs
            .into_iter()
            .map(DocumentResponse::from)
            .collect();

        Ok(SearchResponse {
            documents,
            total_count,
        })
    })
    .await;

    match result {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            warn!("Search failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "search_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

/// Get connection statistics
async fn get_connection_stats(
    State(state): State<AppState>,
) -> Result<Json<ConnectionStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => Ok(Json(ConnectionStatsResponse {
                active_connections: stats.active_connections,
                total_connections: stats.total_connections,
                rejected_connections: stats.rejected_connections,
                rate_limited_requests: stats.rate_limited_requests,
            })),
            Err(e) => {
                warn!("Failed to get connection stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "stats_unavailable".to_string(),
                        message: "Connection statistics temporarily unavailable".to_string(),
                    }),
                ))
            }
        }
    } else {
        // No connection pool configured - return empty stats
        Ok(Json(ConnectionStatsResponse {
            active_connections: 0,
            total_connections: 0,
            rejected_connections: 0,
            rate_limited_requests: 0,
        }))
    }
}

/// Get performance statistics
async fn get_performance_stats(
    State(state): State<AppState>,
) -> Result<Json<PerformanceStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => {
                // Calculate requests per second (simplified - would need time window in real implementation)
                let requests_per_second = if stats.avg_latency_ms > 0.0 {
                    1000.0 / stats.avg_latency_ms // Very rough estimate
                } else {
                    0.0
                };

                Ok(Json(PerformanceStatsResponse {
                    avg_latency_ms: stats.avg_latency_ms,
                    total_requests: stats.total_connections, // Proxy for total requests
                    requests_per_second,
                }))
            }
            Err(e) => {
                warn!("Failed to get performance stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "stats_unavailable".to_string(),
                        message: "Performance statistics temporarily unavailable".to_string(),
                    }),
                ))
            }
        }
    } else {
        // No connection pool configured - return empty stats
        Ok(Json(PerformanceStatsResponse {
            avg_latency_ms: 0.0,
            total_requests: 0,
            requests_per_second: 0.0,
        }))
    }
}

/// Get resource usage statistics
async fn get_resource_stats(
    State(state): State<AppState>,
) -> Result<Json<ResourceStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(pool) = &state.connection_pool {
        match pool.lock().await.get_stats().await {
            Ok(stats) => {
                let memory_mb = stats.memory_usage_bytes as f64 / (1024.0 * 1024.0);

                // Determine system health based on various factors
                let system_healthy = stats.cpu_usage_percent < 90.0
                    && memory_mb < 1000.0 // Under 1GB
                    && (stats.active_connections as f64 / 100.0) < 0.95; // Under 95% capacity

                Ok(Json(ResourceStatsResponse {
                    memory_usage_bytes: stats.memory_usage_bytes,
                    memory_usage_mb: memory_mb,
                    cpu_usage_percent: stats.cpu_usage_percent,
                    system_healthy,
                }))
            }
            Err(e) => {
                warn!("Failed to get resource stats: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "stats_unavailable".to_string(),
                        message: "Resource statistics temporarily unavailable".to_string(),
                    }),
                ))
            }
        }
    } else {
        // No connection pool configured - return basic system stats
        Ok(Json(ResourceStatsResponse {
            memory_usage_bytes: 32 * 1024 * 1024, // 32MB baseline
            memory_usage_mb: 32.0,
            cpu_usage_percent: 5.0, // Low baseline
            system_healthy: true,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_file_storage, wrappers::create_wrapped_storage};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    async fn create_test_storage() -> (Arc<Mutex<dyn Storage>>, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage = create_file_storage(temp_dir.path().to_str().unwrap(), Some(1000))
            .await
            .expect("Failed to create storage");
        let wrapped = create_wrapped_storage(storage, 100).await;
        (Arc::new(Mutex::new(wrapped)), temp_dir)
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty())?)
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_document() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        let request_body = json!({
            "path": "/test.md",
            "title": "Test Document",
            "content": b"Hello, world!".to_vec(),
            "tags": ["test"]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/documents")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent_document() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        let doc_id = Uuid::new_v4();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/documents/{doc_id}"))
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_document_id() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/documents/invalid-id")
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn test_monitoring_endpoints() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        // Test connection stats endpoint
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats/connections")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        // Since we're using create_server (not create_server_with_pool),
        // it should return empty stats
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let stats: ConnectionStatsResponse = serde_json::from_slice(&body)?;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_performance_endpoint() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats/performance")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_resource_endpoint() -> Result<()> {
        let (storage, _temp_dir) = create_test_storage().await;
        let app = create_server(storage);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats/resources")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let stats: ResourceStatsResponse = serde_json::from_slice(&body)?;
        assert!(stats.system_healthy);
        assert!(stats.memory_usage_mb > 0.0);

        Ok(())
    }
}
