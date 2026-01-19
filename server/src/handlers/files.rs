//! File upload/download handlers

use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::DateTime;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::{
    error::{AppError, Result},
    models::*,
    AppState,
};

/// Parse datetime string to timestamp
fn parse_datetime_to_timestamp(s: &str) -> i64 {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp())
}

use super::AuthUser;

/// Upload an encrypted file
pub async fn upload_file(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<FileUploadResponse>> {
    let max_size = state.config.limits.max_file_size_mb * 1024 * 1024;
    let files_path = PathBuf::from(&state.config.storage.files_path);

    // Ensure files directory exists
    fs::create_dir_all(&files_path).await?;

    let mut file_name = String::new();
    let mut mime_type = String::from("application/octet-stream");
    let mut encryption_key_hash = String::new();
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to read multipart: {}", e))
    })? {
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "file" => {
                file_name = field
                    .file_name()
                    .unwrap_or("unnamed")
                    .to_string();

                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }

                let data = field.bytes().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read file: {}", e))
                })?;

                if data.len() as u64 > max_size {
                    return Err(AppError::FileTooLarge);
                }

                file_data = Some(data.to_vec());
            }
            "encryption_key_hash" => {
                encryption_key_hash = field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read encryption_key_hash: {}", e))
                })?;
            }
            _ => {}
        }
    }

    let data = file_data.ok_or(AppError::BadRequest("No file provided".to_string()))?;

    if encryption_key_hash.is_empty() {
        return Err(AppError::BadRequest("encryption_key_hash required".to_string()));
    }

    // Create file metadata
    let file_id = state
        .storage
        .create_file_metadata(
            &auth.user_id,
            &file_name,
            data.len() as i64,
            &mime_type,
            &encryption_key_hash,
            state.config.storage.max_file_age_hours as i64,
        )
        .await?;

    // Save file to disk
    let file_path = files_path.join(&file_id);
    let mut file = fs::File::create(&file_path).await?;
    file.write_all(&data).await?;
    file.flush().await?;

    tracing::info!(
        "File uploaded: id={}, name={}, size={}",
        file_id,
        file_name,
        data.len()
    );

    let metadata = state
        .storage
        .get_file_metadata(&file_id)
        .await?
        .ok_or(AppError::Internal(anyhow::anyhow!("File not found after creation")))?;

    Ok(Json(FileUploadResponse {
        file_id,
        upload_url: None, // Direct upload, no URL needed
        expires_at: parse_datetime_to_timestamp(&metadata.expires_at),
    }))
}

/// Download an encrypted file
pub async fn download_file(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(file_id): Path<String>,
) -> Result<Response> {
    // Get file metadata
    let metadata = state
        .storage
        .get_file_metadata(&file_id)
        .await?
        .ok_or(AppError::NotFound("File not found".to_string()))?;

    // Read file from disk
    let file_path = PathBuf::from(&state.config.storage.files_path).join(&file_id);

    if !file_path.exists() {
        return Err(AppError::NotFound("File not found".to_string()));
    }

    let data = fs::read(&file_path).await?;

    // Increment download count
    state.storage.increment_download_count(&file_id).await?;

    // Build response
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &metadata.mime_type)
        .header(header::CONTENT_LENGTH, data.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", metadata.file_name),
        )
        .header("X-Encryption-Key-Hash", &metadata.encryption_key_hash)
        .body(Body::from(data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Delete a file
pub async fn delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    // Get file metadata
    let metadata = state
        .storage
        .get_file_metadata(&file_id)
        .await?
        .ok_or(AppError::NotFound("File not found".to_string()))?;

    // Only uploader can delete
    if metadata.uploader_id != auth.user_id {
        return Err(AppError::Forbidden);
    }

    // Delete from disk
    let file_path = PathBuf::from(&state.config.storage.files_path).join(&file_id);
    if file_path.exists() {
        fs::remove_file(&file_path).await?;
    }

    // Delete metadata
    state.storage.delete_file_metadata(&file_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}
