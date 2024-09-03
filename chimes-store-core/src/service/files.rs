// File Management

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rbatis::rbdc::Uuid;
use salvo::http::cookie::time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::StoreServiceConfig;

use super::starter::MxStoreService;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UploadFileInfo {
    pub file_id: Option<String>,      // 文件ID
    pub source: Option<String>,       // 源文件名，它是文件上传时的文件名
    pub dest_file: Option<String>, // 目标文件名，它是后续由StoreX管理存放的文件名，只包含逻辑存放路径
    pub dest_path: Option<String>, // 目标文件名，它是后续由StoreX管理存放的文件名，包含实际存放路径
    pub file_type: Option<String>, // 文件类型，即文件的后缀名
    pub content_type: Option<String>, // 文件的content-type，自动识别
    #[serde(default)]
    pub file_size: usize, // 文件的大小
    pub access_url: Option<String>, // 文件进行下载访问的URL，由StoreX管理产生
    pub data: Value,
    #[serde(default)]
    pub copied: bool,
}

pub struct FileStoreManager(pub(crate) StoreServiceConfig);

impl FileStoreManager {
    pub fn max_filesize(&self) -> usize {
        self.0.max_filesize.unwrap_or(20) as usize * 1024usize * 1024usize
    }

    pub fn get_store_path(&self) -> PathBuf {
        let path = self
            .0
            .upload_filepath
            .clone()
            .unwrap_or(MxStoreService::get_assets_path() + "/files");
        path.into()
    }

    pub fn read_file(&self, filename: &str) -> Result<Vec<u8>, anyhow::Error> {
        let path = self.get_store_path();
        let mut file = File::open(path.join(filename))?;
        let mut buf = vec![];
        let _ = file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn calc_filename(&self, org_filename: &str) -> String {
        let filename: PathBuf = org_filename.into();
        if let Some(ext) = filename.extension() {
            let fileuuid = Uuid::new().to_lowercase().replace('-', "");
            let filename_out = fileuuid + "." + &ext.to_string_lossy();
            filename_out
        } else {
            String::new()
        }
    }

    pub fn calc_fullname(&self, orgfile: &str, filename: &str) -> String {
        let mut subpath = String::new();
        subpath.push('/');

        if self.0.subfolder_bytype {
            let file: PathBuf = orgfile.into();
            let ext = file
                .extension()
                .map(|f| f.to_str().unwrap_or_default())
                .unwrap_or("unknown");
            subpath.push_str(ext);
            subpath.push('/');
        }

        if self.0.subfolder_bydate {
            let currdate = OffsetDateTime::now_utc();
            subpath.push_str(&format!("{}/", currdate.year()));
            subpath.push_str(&format!("{}/", currdate.month()));
            subpath.push_str(&format!("{}/", currdate.day()));
        }

        subpath.push_str(filename);
        subpath
    }

    pub fn calc_fullpath(&self, orgfile: &str, filename: &str) -> PathBuf {
        let mut subpath = String::new();
        if self.0.subfolder_bytype {
            let file: PathBuf = orgfile.into();
            let ext = file
                .extension()
                .map(|f| f.to_str().unwrap_or_default())
                .unwrap_or("unknown");
            subpath.push_str(ext);
            subpath.push('/');
        }

        if self.0.subfolder_bydate {
            let currdate = OffsetDateTime::now_utc();
            subpath.push_str(&format!("{}/", currdate.year()));
            subpath.push_str(&format!("{}/", currdate.month()));
            subpath.push_str(&format!("{}/", currdate.day()));
        }

        let fullpath = self.get_store_path().join(subpath);
        if let Err(err) = std::fs::create_dir_all(fullpath.clone()) {
            log::info!("Error to create subdirs: {}", err);
        }
        fullpath.join(filename)
    }

    pub fn get_filetype(&self, source: &str) -> Option<String> {
        let path: PathBuf = source.into();
        path.extension()
            .and_then(|f| f.to_str())
            .map(|f| f.to_string())
    }

    pub fn get_access_url(&self, file_id: &str, filename: &str) -> Option<String> {
        let direct = self.0.download_direct;
        self.0.download_prefix.clone().map(|f| {
            if direct {
                format!("{}{}", f, filename)
            } else {
                format!("{}{}", f, file_id)
            }
        })
    }

    pub fn combine_fullpath(&self, filename: &str) -> PathBuf {
        self.get_store_path().join(filename)
    }

    pub fn copy_file(&self, _filename: &Path, _srcfile: &Path) -> Result<usize, anyhow::Error> {
        Ok(0usize)
    }
}
