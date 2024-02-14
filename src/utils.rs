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

use actix_web::{get, HttpResponse, Error};
use serde::Serialize;

use actix_web::error::ErrorNotFound;
use flate2::read::GzDecoder;
use flate2::{write::GzEncoder, Compression};
use serde_json::json;
use std::fs::File;
use std::path::{PathBuf};
use tar::Archive;
use sysinfo::{System, Disks};

const ARCHIVE_BASE_PATH: &str = "entando-data/archives";
const BASE_PATH: &str = "entando-data";

#[derive(Serialize, Debug)]
pub struct EntandoData {
    status: String,
    path: String,
}

#[derive(Serialize)]
struct DiskInfo {
    total_size: u64,
    used_space: u64,
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

    let mut archive_path = PathBuf::new();
    archive_path.push(ARCHIVE_BASE_PATH);
    let archive_name: String = req.match_info().query("filename").parse().unwrap();
    let archive_full_path: String = format!(
        "{}/{}",
        archive_path.as_path().to_str().unwrap(),
        archive_name
    )
    .parse()?;

    if PathBuf::from(&archive_full_path).exists() {
        let tar_gz = File::open(&archive_full_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(BASE_PATH).ok();

        // remove the archive
        fs::remove_file(&archive_full_path)?;

        Ok(HttpResponse::Ok().json(format!("{},{}", archive_name, &archive_full_path)))
    } else {
        Err(ErrorNotFound(json!(EntandoData {
            status: "Ko".to_string(),
            path: "Wrong Path".to_string(),
        })))
    }
}


#[get("/api/v1/utils/diskinfo")]
pub async fn disk_info() -> Result<HttpResponse, Error> {
    let mut system = System::new_all();
    system.refresh_all();
    let mut found_mount = false;
    let mut disk_info = DiskInfo {
        total_size: 0,
        used_space: 0,
    };
    for disk in &mut Disks::new_with_refreshed_list() {
        if disk.mount_point().to_string_lossy() == "/entando-data" {
            disk_info = DiskInfo {
                total_size: disk.total_space(),
                used_space: disk.total_space() - disk.available_space(),
            };
            found_mount = true;
            break;
        };
    };

    if found_mount {
        Ok::<HttpResponse, Error>(HttpResponse::Ok().json(disk_info))
    } else {
        Ok::<HttpResponse, Error>(HttpResponse::NotFound().body("No disk found"))
    }
}

#[get("/api/v1/utils/compress/{filename:.*}")]
pub async fn compress(req: actix_web::HttpRequest) -> Result<HttpResponse, Error> {
    fs::create_dir_all(ARCHIVE_BASE_PATH).expect("unable to create directory");

    let archive = File::create(format!("{}/entando-data.tar.gz", ARCHIVE_BASE_PATH))?;

    let mut path = PathBuf::new();
    path.push(BASE_PATH);
    path.push(
        req.match_info()
            .query("filename")
            .parse::<PathBuf>()
            .unwrap()
            .as_path(),
    );

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
        return Ok(HttpResponse::Ok().json(EntandoData {
            status: "Ok".to_string(),
            path: file.path().to_str().unwrap().to_string(),
        }));
    } else {
        Err(ErrorNotFound(json!(EntandoData {
            status: "Ko".to_string(),
            path: "Wrong Path".to_string(),
        })))
    }
}


