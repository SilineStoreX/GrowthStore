use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::OnceLock};
// use salvo::oapi::{ToResponse, ToSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    service::starter::{load_config, MxStoreService},
    utils::extractor::create_extractor,
};

use super::extractor::FileExtrator;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileTypeMapping {
    pub ext_name: String,
    pub icon: Option<String>,
    pub library: Option<String>, // 从SharedLibrary中加载
    pub factory: Option<String>, // 工厂名称，如果是从library中加载，则为对应的函数名称，如果library为None，则是从指定的工厂对象中进行获取
    pub options: Option<Value>,  // 执行时所需要的参数，采用JSON格式来进行描述
    pub content_type: Option<String>,
    pub document: Option<bool>,
}

impl FileTypeMapping {
    pub fn new_with_type(ext: &str, ct: &str) -> Self {
        Self {
            ext_name: ext.to_owned(),
            content_type: Some(ct.to_owned()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileTypeLoadInfo {
    pub enable_secure: Option<bool>,
    pub file_types: Vec<FileTypeMapping>,
}

pub struct FileTypeHolder(
    bool,
    HashMap<String, FileTypeMapping>,
    HashMap<String, FileExtrator>,
    FileTypeMapping,
);

impl FileTypeHolder {
    pub fn get() -> &'static Self {
        static FILETYPE_MAPPING_HOLDER: OnceLock<FileTypeHolder> = OnceLock::new();
        FILETYPE_MAPPING_HOLDER.get_or_init(|| {
            let config_path: PathBuf = MxStoreService::get_config_path().into();
            let config_file: PathBuf = config_path.join("filetypes.toml");
            let (s, mut h, extr) = match load_config::<FileTypeLoadInfo>(&config_file) {
                Ok(fl) => {
                    let hash = fl
                        .file_types
                        .iter()
                        .map(|f| (f.ext_name.clone(), f.to_owned()))
                        .collect::<HashMap<String, FileTypeMapping>>();

                    let fx_hash = fl
                        .file_types
                        .iter()
                        .map(create_extractor)
                        .filter(|p| p.is_some())
                        .map(|t| {
                            let n = t.unwrap();
                            (n.get_filetype(), n)
                        })
                        .collect::<HashMap<String, FileExtrator>>();
                    log::info!("filetypes.toml loaded successfully.");
                    (false, hash, fx_hash)
                }
                Err(err) => {
                    log::error!("Could not load filetypes.toml {err}");
                    (false, HashMap::new(), HashMap::new())
                }
            };

            for s in Self::init_default_content_tyeps() {
                if !h.contains_key(&s.ext_name) {
                    h.insert(s.ext_name.clone(), s);
                }
            }

            FileTypeHolder(
                s,
                h,
                extr,
                FileTypeMapping::new_with_type("(default)", "application/octet-stream"),
            )
        })
    }

    pub fn is_secure_enabled(&self) -> bool {
        self.0
    }

    fn init_default_content_tyeps() -> Vec<FileTypeMapping> {
        vec![
            FileTypeMapping::new_with_type("jpg", "image/jpeg"),
            FileTypeMapping::new_with_type("png", "image/png"),
            FileTypeMapping::new_with_type("svg", "image/svg+xml"),
            FileTypeMapping::new_with_type("tiff", "image/tiff"),
            FileTypeMapping::new_with_type("tif", "image/tiff"),
            FileTypeMapping::new_with_type("ico", "image/vnd.microsoft.icon"),
            FileTypeMapping::new_with_type("gif", "image/gif"),
            FileTypeMapping::new_with_type("bmp", "image/bmp"),
            FileTypeMapping::new_with_type("ppm", "image/x-portable-pixmap"),
            FileTypeMapping::new_with_type("mp4", "video/mp4"),
            FileTypeMapping::new_with_type("mp3", "audio/mpeg"),
            FileTypeMapping::new_with_type("mpeg", "audio/mpeg"),
            FileTypeMapping::new_with_type("ogg", "video/ogg"),
            FileTypeMapping::new_with_type("wma", "audio/x-ms-wma"),
            FileTypeMapping::new_with_type("wav", "audio/vnd.wave"),
            FileTypeMapping::new_with_type("ra", "audio/vnd.rn-realaudio"),
            FileTypeMapping::new_with_type("qt", "video/quicktime"),
            FileTypeMapping::new_with_type("webm", "video/webm"),
            FileTypeMapping::new_with_type("wmv", "video/x-ms-wmv"),
            FileTypeMapping::new_with_type("txt", "text/plian"),
            FileTypeMapping::new_with_type("csv", "text/csv"),
            FileTypeMapping::new_with_type("js", "text/javascript"),
            FileTypeMapping::new_with_type("xml", "text/xml"),
            FileTypeMapping::new_with_type("c", "text/plian"),
            FileTypeMapping::new_with_type("cpp", "text/plian"),
            FileTypeMapping::new_with_type("rs", "text/plian"),
            FileTypeMapping::new_with_type("go", "text/plian"),
            FileTypeMapping::new_with_type("doc", "application/msword"),
            FileTypeMapping::new_with_type(
                "docx",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            ),
            FileTypeMapping::new_with_type(
                "xlsx",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            FileTypeMapping::new_with_type(
                "pptx",
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            ),
            FileTypeMapping::new_with_type("pps", "application/vnd.ms-powerpoint"),
            FileTypeMapping::new_with_type("ppt", "application/vnd.ms-powerpoint"),
            FileTypeMapping::new_with_type("xla", "application/vnd.ms-excel"),
            FileTypeMapping::new_with_type("xlc", "application/vnd.ms-excel"),
            FileTypeMapping::new_with_type("xll", "application/x-excel"),
            FileTypeMapping::new_with_type("xlm", "application/vnd.ms-excel"),
            FileTypeMapping::new_with_type("xls", "application/vnd.ms-excel"),
            FileTypeMapping::new_with_type("xlt", "application/vnd.ms-excel"),
            FileTypeMapping::new_with_type("xlw", "application/vnd.ms-excel"),
            FileTypeMapping::new_with_type("rar", "application/x-rar-compressed"),
            FileTypeMapping::new_with_type("tar", "application/x-tar"),
            FileTypeMapping::new_with_type("gzip", "application/x-gzip"),
            FileTypeMapping::new_with_type("gz", "application/gzip"),
            FileTypeMapping::new_with_type("zip", "application/zip"),
            FileTypeMapping::new_with_type("pdf", "application/pdf"),
        ]
    }

    pub fn get_filetypes(&self) -> Vec<FileTypeMapping> {
        self.1.values().cloned().collect()
    }

    pub fn get_filetype(&self, name: &str) -> Option<FileTypeMapping> {
        self.1.get(name).map(|f| f.to_owned())
    }

    pub fn get_content_type(&self, name: &str) -> Option<String> {
        self.get_filetype(name)
            .map(|f| f.content_type)
            .unwrap_or(self.3.content_type.clone())
    }

    pub fn is_documentation(&self, name: &str) -> Option<bool> {
        self.get_filetype(name).map(|f| f.document).unwrap_or(None)
    }

    pub fn get_extractor(&'static self, name: &str) -> Option<&'static FileExtrator> {
        let path: PathBuf = name.into();
        let extname = path
            .extension()
            .map(|t| t.to_str().unwrap_or(name))
            .unwrap_or(name);
        self.2.get(extname)
    }
}

#[cfg(test)]
#[test]
pub fn test_write() {
    use serde_json::json;

    let fmt = FileTypeMapping {
        ext_name: "jpg".to_owned(),
        icon: Some("jpg".to_owned()),
        library: Some("test".to_owned()),
        factory: Some("extract".to_owned()),
        options: Some(json!({"det_path": "../models/ddf.onnx", "thread": 2})),
        content_type: Some("text/plain".to_owned()),
        document: Some(false),
    };

    let fmt1 = FileTypeMapping {
        ext_name: "gif".to_owned(),
        icon: Some("gif".to_owned()),
        library: Some("test".to_owned()),
        factory: Some("extract".to_owned()),
        options: Some(json!({"det_path": "../models/ddf.onnx", "thread": 2, "git": "OK"})),
        content_type: Some("image/gif".to_owned()),
        document: Some(false),
    };

    let flt = FileTypeLoadInfo {
        enable_secure: Some(true),
        file_types: vec![fmt, fmt1],
    };

    println!("toml: {}", toml::to_string(&flt).unwrap_or_default());
}
