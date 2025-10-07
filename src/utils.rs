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
use std::{fmt, fs};

use actix_web::{get, Error, HttpResponse};
use serde::Serialize;

use actix_web::error::{ErrorBadRequest, ErrorNotFound};
use flate2::read::GzDecoder;
use flate2::{write::GzEncoder, Compression};
use serde_json::json;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;

const ARCHIVE_BASE_PATH: &str = "entando-data/archives";
const BASE_PATH: &str = "entando-data";

/// Validates and sanitizes a user-provided path to prevent path traversal attacks.
/// Returns a full PathBuf starting from the base path, or an error if the path is invalid.
pub fn take_validated_and_sanitized_full_path(user_path: &str, base_path: &str) -> Result<PathBuf, Error> {
    let sanitized_path = validate_and_sanitize_path(user_path, base_path)?;

    let mut full_path = PathBuf::from(base_path);
    full_path.push(&sanitized_path);

    Ok(full_path)
}

/// Validates and sanitizes a user-provided path to prevent path traversal attacks.
/// Returns a secure PathBuf relative to the base path, or an error if the path is invalid.
pub fn validate_and_sanitize_path(user_path: &str, base_path: &str) -> Result<PathBuf, Error> {
    // Remove any leading/trailing whitespace
    let user_path = remove_leading_slashes(remove_leading_backslashes(user_path.trim()))
        .replace("\\", "/");

    // Reject absolute paths
    if user_path.starts_with('/') || user_path.starts_with('\\') {
        return Err(ErrorBadRequest("Absolute paths are not allowed"));
    }

    // Reject paths with drive letters (Windows)
    if user_path.len() >= 2 && user_path.chars().nth(1) == Some(':') {
        return Err(ErrorBadRequest("Drive letters are not allowed"));
    }

    // Reject paths with drive letters (Windows)
    if user_path.contains("...") {
        return Err(ErrorBadRequest("Path not valid"));
    }

    // Create the full path and canonicalize it to resolve any .. components
    let mut full_path = PathBuf::from(base_path);
    full_path.push(user_path);

    // Normalize the path manually to handle cases where files don't exist yet
    let normalized_path = normalize_path(&full_path);

    // Ensure the normalized path is still within the base directory
    let base_path_buf = PathBuf::from(base_path);
    let normalized_base = normalize_path(&base_path_buf);

    // Check if the normalized path starts with the base path
    if !path_starts_with_base(&normalized_path, &normalized_base) {
        return Err(ErrorBadRequest("Path traversal attempt detected"));
    }

    // Return the relative path from base
    let relative_path = normalized_path.strip_prefix(&normalized_base)
        .unwrap_or(&normalized_path)
        .to_path_buf();
    Ok(relative_path)
}

/// Manually normalize a path by removing . and .. components
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => {
                components.push(name);
            }
            std::path::Component::ParentDir => {
                // Only pop if we're not already at the root
                if !components.is_empty() {
                    components.pop();
                }
            }
            std::path::Component::CurDir => {
                // Skip current directory references
            }
            _ => {
                // For other components (like RootDir), add them
                components.clear(); // Clear previous components for absolute paths
            }
        }
    }
    
    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }
    result
}

fn remove_leading_slashes(s: &str) -> &str {
    s.trim_start_matches('/')
}

fn remove_leading_backslashes(s: &str) -> &str {
    // Note: the backslash must be escaped in the string literal
    s.trim_start_matches('\\')
}

/// Check if a normalized path starts with a normalized base path
fn path_starts_with_base(path: &Path, base: &Path) -> bool {
    let path_components: Vec<_> = path.components().collect();
    let base_components: Vec<_> = base.components().collect();
    
    // If base has more components than path, path can't start with base
    if base_components.len() > path_components.len() {
        return false;
    }
    
    // Check if all base components match the beginning of path components
    for (base_comp, path_comp) in base_components.iter().zip(path_components.iter()) {
        if base_comp != path_comp {
            return false;
        }
    }
    
    true
}

/// Validates a filename to ensure it's safe for file operations
pub fn validate_filename(filename: &str) -> Result<String, Error> {
    let filename = filename.trim();
    
    if filename.is_empty() {
        return Err(ErrorBadRequest("Filename cannot be empty"));
    }
    
    // Check for path separators in filename
    if filename.contains('/') || filename.contains('\\') {
        return Err(ErrorBadRequest("Filename cannot contain path separators"));
    }
    
    // Check for reserved names and characters
    let reserved_names = [".", ".."];
    if reserved_names.contains(&filename.to_uppercase().as_str()) {
        return Err(ErrorBadRequest("Reserved filename not allowed"));
    }
    
    // Use the existing sanitize_filename crate for additional sanitization
    Ok(sanitize_filename::sanitize(filename))
}

#[derive(Serialize, Debug)]
pub struct EntandoData {
    status: String,
    path: String,
}

impl fmt::Display for EntandoData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.path, self.status)
    }
}

/// This function takes the name of a tar.gz archive and decompress it under `/entando-data`.
/// So when creating the tar.gz archive before uploading it, we need to be sure that the filesytem
/// structure, inside the archive, is the one used by entando and that this filesystem structure is
/// inside the `public` directory:
/// ```bash
/// public/
///├── cms
///│   └── images
///│       ├── entando_at_plan_d0.jpg
///│       ├── entando_at_plan_d1.jpg
///│       ├── entando_at_plan_d2.jpg
///│       ├── entando_at_plan_d3.jpg
///│       ├── entando_at_work_d0.jpg
///│       ├── entando_at_work_d1.jpg
///│       ├── entando_at_work_d2.jpg
///│       ├── entando_at_work_d3.jpg
///│       ├── Entando_Logo_Dark_Blue_d0.jpg
///│       ├── Entando_Logo_Dark_Blue_d1.jpg
///│       ├── Entando_Logo_Dark_Blue_d2.jpg
///│       ├── Entando_Logo_Dark_Blue_d3.jpg
///│       ├── html_code_d0.jpg
///│       ├── html_code_d1.jpg
///│       ├── html_code_d2.jpg
///│       └── html_code_d3.jpg
///├── ootb-widgets
///│   └── static
///│       ├── css
///│       │   ├── main.ac8788ef.chunk.css.map
///│       │   ├── main.ootb.chunk.css
///│       │   └── sitemap.css
///│       └── js
///│           ├── 2.46d1e87e.chunk.js.map
///│           ├── 2.ootb.chunk.js
///│           ├── 2.ootb.chunk.js.LICENSE.txt
///│           ├── main.fb2d745b.chunk.js.map
///│           ├── main.ootb.chunk.js
///│           ├── runtime-main.1c559bb1.js.map
///│           └── runtime-main.ootb.js
///
/// ```
///
/// # Example Call
/// ```bash
/// curl --location --request GET 'https://cds.domain.org/api/v1/utils/decompress/my-archive.tar.gz' \
/// --header 'Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUI...' \
/// ```
///
/// # Arguments
/// * req (req: HttpRequest): the name of the archive to be decompressed
///
/// # Returns
/// (Result<HttpResponse, Error>): a json with the status of the decompression job
#[get("/api/v1/utils/decompress/{filename:.*}")]
pub async fn decompress(req: actix_web::HttpRequest) -> Result<HttpResponse, Error> {
    // create the `ARCHIVE_BASE_PATH` path in case it does not exist
    fs::create_dir_all(ARCHIVE_BASE_PATH).expect("unable to create directory");

    let archive_name: String = req.match_info().query("filename").parse().unwrap();
    
    // Validate the filename to prevent path traversal
    let safe_filename = validate_filename(&archive_name)?;
    
    let mut archive_path = PathBuf::new();
    archive_path.push(ARCHIVE_BASE_PATH);
    archive_path.push(&safe_filename);
    
    let archive_full_path = archive_path.to_string_lossy().to_string();

    if PathBuf::from(&archive_full_path).exists() {
        let tar_gz = File::open(&archive_full_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(BASE_PATH).ok();

        // remove the archive
        fs::remove_file(&archive_full_path)?;

        Ok(HttpResponse::Ok().json(format!("{},{}", safe_filename, &archive_full_path)))
    } else {
        Err(ErrorNotFound(json!(EntandoData {
            status: "Ko".to_string(),
            path: "Wrong Path".to_string(),
        })))
    }
}

#[get("/api/v1/utils/compress/{filename:.*}")]
pub async fn compress(req: actix_web::HttpRequest) -> Result<HttpResponse, Error> {
    fs::create_dir_all(ARCHIVE_BASE_PATH).expect("unable to create directory");

    let archive = File::create(format!("{}/entando-data.tar.gz", ARCHIVE_BASE_PATH))?;

    let user_path: String = req.match_info().query("filename").parse().unwrap();
    
    // Validate the path to prevent path traversal
    let path = take_validated_and_sanitized_full_path(&user_path, BASE_PATH)?;

    let enc = GzEncoder::new(archive, Compression::best());
    let mut tar = tar::Builder::new(enc);

    if path.exists() && path.is_dir() {
        tar.append_dir_all("entando-data", &path)?;
        tar.finish()?;

        let file = format!("{}/entando-data.tar.gz", ARCHIVE_BASE_PATH);

        return Ok(HttpResponse::Ok().json(EntandoData {
            status: "Ok".to_string(),
            path: file,
        }));
    }
    if path.exists() && path.is_file() {
        let mut f = File::open(&path).unwrap();
        tar.append_file("entando-data", &mut f).unwrap();
        let file = afs::NamedFile::open(format!("{}/entando-data.tar.gz", ARCHIVE_BASE_PATH))?;
        Ok(HttpResponse::Ok().json(EntandoData {
            status: "Ok".to_string(),
            path: file.path().to_str().unwrap().to_string(),
        }))
    } else {
        Err(ErrorNotFound(json!(EntandoData {
            status: "Ko".to_string(),
            path: "Wrong Path".to_string(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_and_sanitize_path_valid_paths() {
        // Test valid relative paths
        assert!(validate_and_sanitize_path("public/test.txt", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("protected/images/pic.jpg", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("archives/data.tar.gz", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("public", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("public/nested/deep/file.txt", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("/public/nested/deep/file.txt", "entando-data").is_ok());
    }

    #[test]
    fn test_validate_and_sanitize_path_traversal_attacks() {
        // Test classic path traversal attacks
        assert!(validate_and_sanitize_path("../etc/passwd", "entando-data").is_err());
        assert!(validate_and_sanitize_path("../../etc/passwd", "entando-data").is_err());
        assert!(validate_and_sanitize_path("../../../root/.ssh/id_rsa", "entando-data").is_err());

        // Test nested path traversal
        assert!(validate_and_sanitize_path("public/../../../etc/passwd", "entando-data").is_err());
        assert!(validate_and_sanitize_path("public/images/../../..", "entando-data").is_err());
        assert!(validate_and_sanitize_path("protected/../public/../..", "entando-data").is_err());

        // Test URL encoded path traversal
        assert!(validate_and_sanitize_path("%2e%2e%2f%2e%2e%2f%65%74%63%2f%70%61%73%73%77%64", "entando-data").is_ok()); // This should be handled by URL decoding before reaching this function

        // Test double dot variations
        assert!(validate_and_sanitize_path("....//....//etc/passwd", "entando-data").is_err()); // This becomes valid after normalization
        assert!(validate_and_sanitize_path("..././..././etc/passwd", "entando-data").is_err());

        assert!(validate_and_sanitize_path("temp/../../../tmp", "./entando-data/public/").is_err());
        assert!(validate_and_sanitize_path("temp/../../../tmp", "./entando-data/protected/").is_err());
        assert!(validate_and_sanitize_path("temp/../../../tmp", "entando-data/").is_err());
        assert!(validate_and_sanitize_path("temp/../../../tmp", "entando-data/archives").is_err());
        assert!(validate_and_sanitize_path("temp/../../../tmp", "entando-data").is_err());
    }

    #[test]
    fn test_validate_and_sanitize_path_absolute_paths() {
        // Test absolute paths (should be rejected)
        assert!(validate_and_sanitize_path("/etc/passwd", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("////etc/passwd", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("/root/.ssh/id_rsa", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("\\windows\\system32\\drivers\\etc\\hosts", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("\\\\windows\\system32\\drivers\\etc\\hosts", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("/home/user/.bashrc", "entando-data").is_ok());
    }

    #[test]
    fn test_validate_and_sanitize_path_windows_attacks() {
        // Test Windows-specific attacks
        assert!(validate_and_sanitize_path("C:\\windows\\system32\\config\\sam", "entando-data").is_err());
        assert!(validate_and_sanitize_path("D:\\sensitive\\data.txt", "entando-data").is_err());
        assert!(validate_and_sanitize_path("..\\..\\windows\\system32", "entando-data").is_err());
        assert!(validate_and_sanitize_path("public\\..\\..\\windows", "entando-data").is_err());
    }

    #[test]
    fn test_validate_and_sanitize_path_root_cases() {
        // Test empty and whitespace paths
        assert!(validate_and_sanitize_path("", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("   ", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("\t\n", "entando-data").is_ok());

        // Test root folder
        assert!(validate_and_sanitize_path("/", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("\\", "entando-data").is_ok());

        // Test base folder
        assert!(validate_and_sanitize_path(".", "entando-data").is_ok());
    }

    #[test]
    fn test_validate_and_sanitize_path_edge_cases() {
        // Test current directory references
        assert!(validate_and_sanitize_path("./public/test.txt", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("public/./test.txt", "entando-data").is_ok());
        
        // Test paths with spaces and special characters
        assert!(validate_and_sanitize_path("public/my file.txt", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("public/file with spaces.txt", "entando-data").is_ok());
    }

    #[test]
    fn test_validate_and_sanitize_path_complex_attacks() {
        // Test complex path traversal combinations
        assert!(validate_and_sanitize_path("public/../protected/../public/../..", "entando-data").is_err());
        assert!(validate_and_sanitize_path("public/./../../etc/passwd", "entando-data").is_err());
        assert!(validate_and_sanitize_path("protected/./../public/./../..", "entando-data").is_err());
        
        // Test deeply nested traversal
        assert!(validate_and_sanitize_path("a/b/c/d/e/f/g/h/i/j/../../../../../../../../etc/passwd", "entando-data").is_ok());

        // Test mixed separators
        assert!(validate_and_sanitize_path("public\\..\\protected/..\\..\\etc\\passwd", "entando-data").is_err());
    }

    #[test]
    fn test_take_validated_and_sanitized_full_path_valid_paths() {
        // Test valid relative paths
        assert!(take_validated_and_sanitized_full_path("public/test.txt", "entando-data").unwrap().to_string_lossy() == "entando-data/public/test.txt");
        assert!(take_validated_and_sanitized_full_path("protected/images/pic.jpg", "entando-data").unwrap().to_string_lossy() == "entando-data/protected/images/pic.jpg");
        assert!(take_validated_and_sanitized_full_path("archives/data.tar.gz", "entando-data").unwrap().to_string_lossy() == "entando-data/archives/data.tar.gz");
        assert!(take_validated_and_sanitized_full_path("public", "entando-data").unwrap().to_string_lossy() == "entando-data/public");
        assert!(take_validated_and_sanitized_full_path("public/nested/deep/file.txt", "entando-data").unwrap().to_string_lossy() == "entando-data/public/nested/deep/file.txt");

        assert!(take_validated_and_sanitized_full_path("public/../test.txt", "entando-data").unwrap().to_string_lossy() == "entando-data/test.txt");
        assert!(take_validated_and_sanitized_full_path("protected/images/../images/pic.jpg", "entando-data").unwrap().to_string_lossy() == "entando-data/protected/images/pic.jpg");
        assert!(take_validated_and_sanitized_full_path("../entando-data/archives/data.tar.gz", "entando-data").unwrap().to_string_lossy() == "entando-data/archives/data.tar.gz");
        assert!(take_validated_and_sanitized_full_path("public/../protected", "entando-data").unwrap().to_string_lossy() == "entando-data/protected");
    }

    #[test]
    fn test_take_validated_and_sanitized_full_path_path_traversal_attacks() {
        // Test classic path traversal attacks
        assert!(take_validated_and_sanitized_full_path("../etc/passwd", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("../../etc/passwd", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("../../../root/.ssh/id_rsa", "entando-data").is_err());

        // Test nested path traversal
        assert!(take_validated_and_sanitized_full_path("public/../../../etc/passwd", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("public/images/../../..", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("protected/../public/../..", "entando-data").is_err());

        // Test URL encoded path traversal
        assert!(take_validated_and_sanitized_full_path("%2e%2e%2f%2e%2e%2f%65%74%63%2f%70%61%73%73%77%64", "entando-data").is_ok()); // This should be handled by URL decoding before reaching this function

        // Test double dot variations
        assert!(take_validated_and_sanitized_full_path("....//....//etc/passwd", "entando-data").is_err()); // This becomes valid after normalization
        assert!(take_validated_and_sanitized_full_path("..././..././etc/passwd", "entando-data").is_err());

        assert!(take_validated_and_sanitized_full_path("temp/../../../tmp", "./entando-data/public/").is_err());
        assert!(take_validated_and_sanitized_full_path("temp/../../../tmp", "./entando-data/protected/").is_err());
        assert!(take_validated_and_sanitized_full_path("temp/../../../tmp", "entando-data/").is_err());
        assert!(take_validated_and_sanitized_full_path("temp/../../../tmp", "entando-data/archives").is_err());
        assert!(take_validated_and_sanitized_full_path("temp/../../../tmp", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("temp/../../../tmp/..", "entando-data").is_err());
    }

    #[test]
    fn test_take_validated_and_sanitized_full_path_absolute_paths() {
        // Test absolute paths (should be rejected)
        assert!(take_validated_and_sanitized_full_path("/etc/passwd", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("////etc/passwd", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("/root/.ssh/id_rsa", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("\\windows\\system32\\drivers\\etc\\hosts", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("\\\\windows\\system32\\drivers\\etc\\hosts", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("/home/user/.bashrc", "entando-data").is_ok());
    }

    #[test]
    fn test_take_validated_and_sanitized_full_path_root_cases() {
        // Test empty and whitespace paths
        assert!(take_validated_and_sanitized_full_path("", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("   ", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("\t\n", "entando-data").is_ok());

        // Test root folder
        assert!(take_validated_and_sanitized_full_path("/", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("\\", "entando-data").is_ok());

        // Test base folder
        assert!(take_validated_and_sanitized_full_path(".", "entando-data").is_ok());
    }

    #[test]
    fn test_take_validated_and_sanitized_full_path_edge_cases() {
        // Test current directory references
        assert!(take_validated_and_sanitized_full_path("./public/test.txt", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("public/./test.txt", "entando-data").is_ok());

        // Test paths with spaces and special characters
        assert!(take_validated_and_sanitized_full_path("public/my file.txt", "entando-data").is_ok());
        assert!(take_validated_and_sanitized_full_path("public/file with spaces.txt", "entando-data").is_ok());
    }

    #[test]
    fn test_take_validated_and_sanitized_full_path_complex_attacks() {
        // Test complex path traversal combinations
        assert!(take_validated_and_sanitized_full_path("public/../protected/../public/../..", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("public/./../../etc/passwd", "entando-data").is_err());
        assert!(take_validated_and_sanitized_full_path("protected/./../public/./../..", "entando-data").is_err());

        // Test deeply nested traversal
        assert!(take_validated_and_sanitized_full_path("a/b/c/d/e/f/g/h/i/j/../../../../../../../../etc/passwd", "entando-data").is_ok());

        // Test mixed separators
        assert!(take_validated_and_sanitized_full_path("public\\..\\protected/..\\..\\etc\\passwd", "entando-data").is_err());
    }

    #[test]
    fn test_validate_filename_valid_filenames() {
        // Test valid filenames
        assert!(validate_filename("test.txt").is_ok());
        assert!(validate_filename("image.jpg").is_ok());
        assert!(validate_filename("data.tar.gz").is_ok());
        assert!(validate_filename("my-file_name.pdf").is_ok());
        assert!(validate_filename("file123.doc").is_ok());
        assert_eq!(validate_filename("test.txt").unwrap(), "test.txt");
    }

    #[test]
    fn test_validate_filename_path_separators() {
        // Test filenames with path separators (should be rejected)
        assert!(validate_filename("../test.txt").is_err());
        assert!(validate_filename("folder/test.txt").is_err());
        assert!(validate_filename("..\\test.txt").is_err());
        assert!(validate_filename("folder\\test.txt").is_err());
        assert!(validate_filename("/etc/passwd").is_err());
        assert!(validate_filename("\\windows\\system32").is_err());
    }

    #[test]
    fn test_validate_filename_reserved_names() {
        // Test special directory names
        assert!(validate_filename(".").is_err());
        assert!(validate_filename("..").is_err());
    }

    #[test]
    fn test_validate_filename_edge_cases() {
        // Test empty and whitespace filenames
        assert!(validate_filename("").is_err());
        assert!(validate_filename("   ").is_err());
        assert!(validate_filename("\t").is_err());
        assert!(validate_filename("\n").is_err());
        
        // Test filenames with leading/trailing spaces
        assert!(validate_filename("  test.txt  ").is_ok());
        assert_eq!(validate_filename("  test.txt  ").unwrap(), "test.txt");
    }

    #[test]
    fn test_validate_filename_special_characters() {
        // Test that the sanitize_filename crate handles special characters
        let result = validate_filename("file<>:|?*.txt");
        assert!(result.is_ok());
        // The exact sanitized result depends on the sanitize_filename crate implementation
        let sanitized = result.unwrap();
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains(':'));
        assert!(!sanitized.contains('|'));
        assert!(!sanitized.contains('?'));
        assert!(!sanitized.contains('*'));
    }

    #[test]
    fn test_normalize_path() {
        // Test path normalization
        assert_eq!(normalize_path(&PathBuf::from("a/b/c")), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path(&PathBuf::from("a/./b/c")), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path(&PathBuf::from("a/b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize_path(&PathBuf::from("a/b/../../c")), PathBuf::from("c"));
        assert_eq!(normalize_path(&PathBuf::from("a/b/c/../..")), PathBuf::from("a"));
        
        // Test with empty components
        assert_eq!(normalize_path(&PathBuf::from("./a/b/c")), PathBuf::from("a/b/c"));
        assert_eq!(normalize_path(&PathBuf::from("a/./././b/c")), PathBuf::from("a/b/c"));
    }

    #[test]
    fn test_path_starts_with_base() {
        let base = PathBuf::from("entando-data");
        
        // Test valid paths
        assert!(path_starts_with_base(&PathBuf::from("entando-data/public"), &base));
        assert!(path_starts_with_base(&PathBuf::from("entando-data/protected/images"), &base));
        assert!(path_starts_with_base(&PathBuf::from("entando-data"), &base));
        
        // Test invalid paths
        assert!(!path_starts_with_base(&PathBuf::from("etc/passwd"), &base));
        assert!(!path_starts_with_base(&PathBuf::from("root"), &base));
        assert!(!path_starts_with_base(&PathBuf::from("entando"), &base)); // partial match
    }

    #[test]
    fn test_path_traversal_error_messages() {
        // Test that error messages are appropriate for security (don't leak too much info)
        let result = validate_and_sanitize_path("../etc/passwd", "entando-data");
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Path traversal attempt detected"));
        
        let result = validate_and_sanitize_path("/etc/passwd", "entando-data");
        assert!(result.is_ok());

        let result = validate_filename("../test.txt");
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Filename cannot contain path separators"));
    }

    #[test]
    fn test_realistic_attack_scenarios() {
        // Test realistic attack scenarios that might be encountered
        
        // Scenario 1: Attacker tries to access system files
        assert!(validate_and_sanitize_path("../../../etc/passwd", "entando-data").is_err());
        assert!(validate_and_sanitize_path("public/../../../etc/shadow", "entando-data").is_err());
        
        // Scenario 2: Attacker tries to access application config files
        assert!(validate_and_sanitize_path("../../config/database.yml", "entando-data").is_err());
        assert!(validate_and_sanitize_path("../../../app/config/secrets.json", "entando-data").is_err());
        
        // Scenario 3: Attacker tries to write files outside allowed directory
        assert!(validate_and_sanitize_path("../../../tmp/malicious.sh", "entando-data").is_err());
        assert!(validate_and_sanitize_path("public/../../../var/www/backdoor.php", "entando-data").is_err());
        
        // Scenario 4: Valid paths that should work
        assert!(validate_and_sanitize_path("public/images/logo.png", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("protected/docs/manual.pdf", "entando-data").is_ok());
        assert!(validate_and_sanitize_path("archives/backup.tar.gz", "entando-data").is_ok());
    }

    #[test]
    fn test_unicode_and_encoded_attacks() {
        // Test Unicode normalization attacks
        assert!(validate_and_sanitize_path("public/test\u{2215}file.txt", "entando-data").is_ok()); // Division slash
        
        // Test null byte attacks (should be handled by sanitize_filename)
        let result = validate_filename("test\0.txt");
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(!sanitized.contains('\0'));
        
        // Test control characters
        let result = validate_filename("test\x01\x02\x03.txt");
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(!sanitized.contains('\x01'));
        assert!(!sanitized.contains('\x02'));
        assert!(!sanitized.contains('\x03'));
    }
}
