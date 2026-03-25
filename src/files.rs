use crate::{config::CONFIG, errors::AppError};
use axum::{
    extract::{Multipart, Path},
    http::header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{fs, path};

use rand::random;

/// Upload a file. Returns an `AppError` or the path of the uploaded file.
/// If `filename` param has a value choose it as filename
pub async fn upload(
    mut multipart: Multipart,
    allowed_extensions: Vec<&str>,
    filename: Option<String>,
) -> Result<String, AppError> {
    let mut uploaded_file = String::new();

    if let Some(file) = multipart.next_field().await.unwrap() {
        let content_type = file.content_type().unwrap_or("application/octet-stream");
        let content_type_ext = content_type
            .split('/')
            .nth(1)
            .unwrap_or("octet-stream")
            .to_lowercase();

        let filename_ext = file
            .file_name()
            .and_then(|name| path::Path::new(name).extension())
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        let ext_name = filename_ext.unwrap_or(content_type_ext);

        if ext_name == "octet-stream" {
            return Err(AppError::BadRequest(
                "Unable to detect file extension. Please upload files with a valid extension (.stl, .obj, .png, .jpg, .jpeg, .webp)."
                    .to_string(),
            ));
        }

        if allowed_extensions.iter().any(|&x| x.eq_ignore_ascii_case(&ext_name)) {
            let mut name = match filename {
                Some(name) => name,
                None => (random::<f32>() * 1000000000 as f32).to_string(),
            };

            loop {
                let save_filename =
                    format!("{}/{}.{}", CONFIG.save_file_base_path, name, ext_name);

                if path::Path::exists(&path::Path::new(&save_filename)) {
                    name = (random::<f32>() * 1000000000 as f32).to_string();
                    continue;
                }

                uploaded_file = format!("{}/{}.{}", CONFIG.uploads_endpoint, name, ext_name);

                let data = file.bytes().await.unwrap();

                tokio::fs::write(&save_filename, &data)
                    .await
                    .map_err(|err| err.to_string())?;
                break;
            }
        }
    }

    if !uploaded_file.is_empty() {
        return Ok(uploaded_file);
    }

    Err(AppError::BadRequest(
        "File extension not supported".to_string(),
    ))
}

/// Delete a file from the filesystem
pub fn delete_upload(filename: &str) -> Result<(), AppError> {
    let last_slash_index = filename.rfind('/').unwrap();
    let path = format!(
        "{}/{}",
        CONFIG.save_file_base_path,
        &filename[last_slash_index + 1..]
    );

    fs::remove_file(path)?;

    Ok(())
}

/// Axum endpoint which shows uploaded file
pub async fn show_uploads(Path(id): Path<String>) -> (HeaderMap, Vec<u8>) {
    let index = id.find('.').unwrap_or(usize::max_value());

    let mut ext_name = "xxx";
    if index != usize::max_value() {
        ext_name = &id[index + 1..];
    }
    let ext_name = ext_name.to_lowercase();
    let mut headers = HeaderMap::new();

    let content_type = match ext_name.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "stl" => "model/stl",
        "obj" => "text/plain",
        _ => "application/octet-stream",
    };

    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_str(content_type).unwrap(),
    );

    let content_disposition = format!("attachment; filename=\"{}\"", id);
    headers.insert(
        HeaderName::from_static("content-disposition"),
        HeaderValue::from_str(&content_disposition).unwrap(),
    );

    let file_name = format!("{}/{}", CONFIG.save_file_base_path, id);
    (headers, fs::read(&file_name).unwrap())
}
