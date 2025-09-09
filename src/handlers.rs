/*++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 + Copyright (c) 2022 Entando SRL.                                                                 +
 + Permission is hereby granted, free of charge, to any person obtaining a copy of this software   +
 + and associated documentation files (the "Software"), to deal in the Software without            +
 + restriction, including without limitation the rights to use, copy, modify, merge, publish,      +
 + distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the   +
 + Software is furnished to do so, subject to the following conditions:                            +
 +                                                                                                 +
 + The above copyright notice and this permission notice shall be included in all copies or        +
 + substantial portions of the Software.                                                           +
 +                                                                                                 +
 + THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR                      +
 + IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,                        +
 + FITNESS FOR A PARTICULAR PURPOSE AND NON INFRINGEMENT. IN NO EVENT SHALL THE                    +
 + AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER                          +
 + LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,                   +
 + OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE                   +
 + SOFTWARE.                                                                                       +
 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++*/

use actix_files as afs;
use core::fmt;
use std::fs;

use std::time::{SystemTime, UNIX_EPOCH};

use actix_multipart::Multipart;
use actix_web::{body::BoxBody, delete, get, post, web, Error, HttpRequest, HttpResponse, Responder};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

use actix_web::error::{ErrorForbidden, ErrorNotFound};
use serde_json::json;
use std::fmt::Formatter;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use crate::utils::{take_validated_and_sanitized_full_path, validate_and_sanitize_path, validate_filename};


const PUBLIC_UPLOAD_PATH: &str = "./entando-data/public/";
const PROTECTED_UPLOAD_PATH: &str = "./entando-data/protected/";
const BASE_PATH: &str = "entando-data/";

/// This struct defines an health-check response
///
/// # Attributes
/// * status (String): the return value of the REST API call
#[derive(Serialize, Deserialize)]
pub struct HealthCheck {
    status: String,
}

/// This struct defines the status of the delete REST API
///
/// # Attributes
/// * status (String): the return value of the REST API call
#[derive(Serialize)]
pub struct Delete {
    status: String,
}

/// This function returns the status of the CDS service. It's used to define the healthcheck on K8S.
///
/// # Arguments
/// No arguments are needed.
///
/// # Returns
/// (HttpResponse, Error): a json describing the status of the CDS service. Which is always `ok`.
pub async fn health_check() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(&HealthCheck {
        status: "ok".to_string(),
    }))
}

/// This struct defines a FileResource, which is used by the `upload` REST API
///
/// # Attributes
/// * status (String): the result of the call
/// * filename (String): the name of the file being uploaded
/// * file (String): The local path of the file to be uploaded or the stream
/// * date (u64): The UnixTime in seconds
/// * path (String): The path where the file should be copied to the CDS server. If the `archives`
/// value is passed, than a `tar.gz` archive is expected which should be copied inside a specific path
/// `entando-data/archives/[your-archive].tar.gz`. The `filename` attribute must end with tar.gz if
/// `archives` is passed as path.
/// * is_protected_file (String): Accepted values are (true, false). If the value is `true` than the
/// file should be copied inside `/entando-data/protected` directory, otherwise to the
/// `/entando-data/public` one.
#[derive(Serialize, Deserialize)]
pub struct FileResource {
    status: String,
    filename: String, // the name of the file
    file: String,
    date: u64,
    path: String,
    is_protected_file: String,
}

/// This struct defines a PathResource which is used by the `list` REST API
///
/// # Attributes
/// * name (String): the name of the resource
/// * last_modified_time (SystemTime): the last modified date expressed in UnixTime (Seconds)
/// * size (u64): return the size of the resource in bytes
/// * directory (bool): return `true` if the resource is a directory
/// * path (String): the full path of the resource
/// * protected_folder (bool): return `true` if the resource is a protected one
#[derive(Serialize, Debug)]
pub struct PathResource {
    name: String,
    last_modified_time: SystemTime,
    size: u64,
    directory: bool,
    path: String,
    protected_folder: bool,
}

/// This is the Responder implementation for the PathResource struct. We need it to return  json as response
impl Responder for PathResource {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        // Create response and set content type
        HttpResponse::Ok()
            .content_type("application/json")
            .body(body)
    }
}

/// This is the fmt::Display standard implementation for the PathResource struct.
impl fmt::Display for PathResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}",
            self.name,
            self.last_modified_time
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            self.size,
            self.directory,
            self.path,
            self.protected_folder
        )
    }
}

/// This function allows us to upload a file inside the CDS service.
/// Pay attention at the order of the parameters in the body request.
///
/// # Example call
/// ```bash
/// curl --location --request POST 'https://cds.domain.com/api/v1/upload/' \
/// --header 'Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiA...51PKX7WLmOitaO2A' \
/// --form 'path="cms/static/images"' \
/// --form 'protected="false"' \
/// --form 'filename="image.png"' \
/// --form 'file=@"/home/user/Images/Screenshot_20220526_114549.png"'
/// ```
///
/// # Arguments
/// * data (&mut data: Multipart): the multipart-form data
///
/// # Returns
/// (Result<HttpResponse, Error>): a json describing the uploaded file/stream resource
#[post("/api/v1/upload/")]
pub async fn upload(mut data: Multipart) -> Result<HttpResponse, Error> {
    let file = "".to_string();
    let mut filename = "".to_string();
    let mut path_value = "".to_string();
    let mut protected_value = "".to_string();
    let mut results = vec![];
    let mut is_directory: bool = false;
    let mut safe_folder_path = PathBuf::new();
    // let mut status = "".to_string();
    while let Ok(Some(mut param)) = data.try_next().await {
        let content_type = param.content_disposition().clone();
        let param_field = content_type.get_name().unwrap();

        if param_field == "path" {
            while let Some(chunk) = param.try_next().await? {
                path_value = std::str::from_utf8(&chunk).unwrap().to_string();
            }
        }

        if param_field == "protected" {
            while let Some(chunk) = param.try_next().await? {
                protected_value = std::str::from_utf8(&chunk).unwrap().to_string();
            }
            if &path_value == "archives" {
                safe_folder_path = take_validated_and_sanitized_full_path(&path_value, BASE_PATH)?;
            } else if protected_value == "true" {
                safe_folder_path = take_validated_and_sanitized_full_path(&path_value, PROTECTED_UPLOAD_PATH)?;
            } else {
                safe_folder_path = take_validated_and_sanitized_full_path(&path_value, PUBLIC_UPLOAD_PATH)?;
            }
            fs::create_dir_all(safe_folder_path.as_path()).expect("unable to create directory");
        }

        if param_field == "filename" {
            while let Some(chunk) = param.try_next().await? {
                let raw_filename = std::str::from_utf8(&chunk).unwrap().to_string();
                // Validate and sanitize the filename
                filename = if raw_filename.is_empty() {
                    String::new()
                } else {
                    validate_filename(&raw_filename)?
                };
            }
            is_directory = filename.is_empty();
        }

        if param_field == "file" && !is_directory {
            // filename is already validated and sanitized
            let full_file_path = format!("{}/{}", safe_folder_path.to_string_lossy(), filename);
            let mut f = web::block(|| fs::File::create(full_file_path)).await??;
            // param is a stream of bytes
            while let Some(chunk) = param.try_next().await? {
                // let stream = chunk.unwrap();
                f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
            }
        }
        if !file.is_empty() {
            let mut result = vec![FileResource {
                status: "Ok".to_string(),
                filename: String::from(&filename),
                file: "".to_string(),
                date: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                path: safe_folder_path.to_string_lossy().to_string(),
                is_protected_file: protected_value.to_owned(),
            }];

            results.append(&mut result);
        }
        //
    }

    let mut result = vec![FileResource {
        status: "Ok".to_string(),
        filename,
        file,
        date: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        path: safe_folder_path.to_string_lossy().to_string(),
        is_protected_file: protected_value.to_owned(),
    }];

    results.append(&mut result);

    Ok(HttpResponse::Ok().json(results))
}

/// This function returns the passed file resource and is the public interface exposed by Ingress.
///
/// # Example Call
///
/// ```bash
/// curl -v https://cds.domain.com/public/my-file.txt
/// ```
///
/// # Arguments
/// * req (req: web::Path<(String, String): the query string request.
/// {tenant} - deserialize to a String
/// {filename} - deserialize to a String
///
/// # Returns
/// (Result<afs:NamedFile, Error>): the file resource requested
#[get("/{tenant}/{filename:.*}")]
pub async fn index(req: web::Path<(String, String)>) -> Result<afs::NamedFile, Error> {
    let (_tenant, filename) = req.into_inner();
    if filename.starts_with("public/") || filename.starts_with("archives/") {
        // Validate and sanitize the path to prevent path traversal
        let path = take_validated_and_sanitized_full_path(&filename, BASE_PATH)?;
        if path.exists() && path.is_file() {
            let file = afs::NamedFile::open(path)?;
            Ok(file.use_etag(true).use_last_modified(true))
        } else {
            Err(ErrorNotFound(
                "File not found. Or tried to list content of a directory.",
            ))
        }
    } else {
        Err(ErrorForbidden(
            "You are not allowed to get this protected resource",
        ))
    }
}

/// This function returns the passed file resource and is using the protected interface.
///
/// # Example Call
///
/// ```bash
/// curl --location --request GET 'http://cds:8080/api/v1/protected/my-file.txt' \
/// --header 'Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAi...'
/// ```
///
/// # Arguments
/// * req (req: HttpRequest): the query string request
///
/// # Returns
/// (Result<afs:NamedFile, Error>): the file resource requested
#[get("/api/v1/{filename:.*}")]
pub async fn index_protected(req: HttpRequest) -> Result<afs::NamedFile, Error> {
    let filename: String = req.match_info().query("filename").parse().unwrap();
    
    // Validate and sanitize the path to prevent path traversal
    let path = take_validated_and_sanitized_full_path(&filename, BASE_PATH)?;
    if path.exists() && path.is_file() {
        let file = afs::NamedFile::open(path)?;
        Ok(file.use_etag(true).use_last_modified(true))
    } else {
        Err(ErrorNotFound(
            "File not found. Or tried to list content of a directory.",
        ))
    }
}

/// This function delete the file resource passed as parameter. The file resource could be a single
/// file or a path. If it is path, the entire content of that path will be deleted.
///
/// # Example Call
/// ```bash
/// curl --location --request DELETE 'https://cds.domain.com/api/v1/delete/public/test.txt' \
/// --header 'Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAi...'
/// ```
///
/// # Arguments
/// * req (req: HttpRequest): the path of the file resource to be deleted
///
/// # Returns
/// (Result<HttpResponse, Error>): a json returning the status of the operation
#[delete("/api/v1/delete/{filename:.*}")]
pub async fn delete(req: HttpRequest) -> Result<HttpResponse, Error> {
    let filename: String = req.match_info().query("filename").parse().unwrap();
    
    // Validate and sanitize the path to prevent path traversal
    let path = take_validated_and_sanitized_full_path(&filename, BASE_PATH)?;

    let result = if path.exists() {
        if path.is_dir() {
            fs::remove_dir_all(&path)?; // Warning this will remove all the contents
        } else {
            fs::remove_file(&path)?;
        }
        true
    } else {
        false
    };

    match result {
        true => Ok(HttpResponse::Ok().json(&Delete {
            status: "OK".to_string(),
        })),
        false => Ok(HttpResponse::Ok().json(&Delete {
            status: "KO".to_string(),
        })),
    }
}

/// This function returns a json containing the contents of the given path
///
/// # Example Call
/// ```bash
/// curl --location --request GET 'https://cds.domain.com/api/v1/list/' \
/// --header 'Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSl...-cDJppc5bmdAA' \
///
/// Returns:
///
/// [
///     {
///         "name": "public",
///         "last_modified_time": {
///             "secs_since_epoch": 1656516728,
///             "nanos_since_epoch": 105205283
///         },
///         "size": 17,
///         "directory": true,
///         "path": "entando-data/public",
///         "protected_folder": true
///     },
///     {
///         "name": "protected",
///         "last_modified_time": {
///             "secs_since_epoch": 1656516727,
///             "nanos_since_epoch": 710202549
///         },
///         "size": 38,
///         "directory": true,
///         "path": "entando-data/protected",
///         "protected_folder": true
///     }
/// ]
/// ```
///
/// # Arguments
/// * req (req: HttpRequest): the path
///
/// # Returns
/// * (Result<HttpResponse, Error>: the json describing the filesystem structure of the requested path
/// with some metadata
#[get("/api/v1/list/{filename:.*}")]
pub async fn list(req: HttpRequest) -> Result<HttpResponse, Error> {
    let filename: String = req.match_info().query("filename").parse().unwrap();
    
    // Validate and sanitize the path to prevent path traversal
    let safe_path = validate_and_sanitize_path(&filename, BASE_PATH)?;
    
    let mut path = PathBuf::from(BASE_PATH);
    path.push(&safe_path);

    if path.exists() {
        let mut protected: bool = true;
        if safe_path.starts_with("public") {
            protected = false
        }

        if path.is_file() {
            let result = vec![PathResource {
                name: path
                    .file_name()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .unwrap(),
                last_modified_time: path.metadata().unwrap().modified().unwrap(),
                size: path.metadata().unwrap().size(),
                directory: false,
                path: safe_path.to_string_lossy().to_string(),
                protected_folder: false,
            }];

            Ok(HttpResponse::Ok().json(result))
        } else {
            let entries = fs::read_dir(path).unwrap();
            let mut results = vec![];
            for entry in entries {
                let mut result = vec![PathResource {
                    name: entry.as_ref().unwrap().file_name().into_string().unwrap(),
                    last_modified_time: entry
                        .as_ref()
                        .unwrap()
                        .metadata()
                        .unwrap()
                        .modified()
                        .unwrap(),
                    size: entry.as_ref().unwrap().metadata().unwrap().size(),
                    directory: entry.as_ref().unwrap().metadata().unwrap().is_dir(),
                    path: entry
                        .as_ref()
                        .unwrap()
                        .path()
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                    protected_folder: protected,
                }];
                results.append(&mut result);
            }
            Ok(HttpResponse::Ok().json(results))
        }
    } else {
        Err(ErrorNotFound(json!(vec![PathResource {
            name: "Wrong_path".to_string(),
            last_modified_time: SystemTime::now(),
            size: 0,
            directory: false,
            path: "".to_string(),
            protected_folder: false,
        }])))
    }
}
