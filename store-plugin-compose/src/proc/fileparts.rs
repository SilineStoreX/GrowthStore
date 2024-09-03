use std::str::FromStr;

use chimes_store_core::service::files::{FileStoreManager, UploadFileInfo};
use salvo::http::{form::FilePart, Mime};
use serde_json::{Map, Value};

pub fn process_filepart(
    fm: &FileStoreManager,
    filepart: &FilePart,
    fields: &Map<String, Value>,
) -> UploadFileInfo {
    let file_id = uuid::Uuid::new_v4().to_string().replace('-', "");
    let filesize = filepart.size() as usize;
    let source = filepart.name().unwrap_or_default();
    let ftype = fm.get_filetype(source);
    let filename = fm.calc_filename(source);
    let dest_name = fm.calc_fullname(source, &filename);
    let dest_file = fm.calc_fullpath(source, &filename);
    let mime = filepart
        .content_type()
        .unwrap_or(Mime::from_str("application/octet-stream").unwrap());
    let down_url = fm.get_access_url(&file_id, &dest_name);
    let source_path = filepart.path(); //.with_file_name(source);
    log::info!("Source: {:?}", source_path);
    log::info!("Dest: {:?}", dest_file);
    if filesize > fm.max_filesize() {
        return UploadFileInfo {
            file_size: filesize,
            copied: false,
            ..Default::default()
        };
    }
    if let Err(err) = std::fs::copy(source_path, dest_file.clone()) {
        log::info!("copy file with error {}", err);
        UploadFileInfo {
            file_id: Some(file_id),
            source: Some(source.to_string()),
            dest_file: Some(dest_name),
            dest_path: dest_file.to_str().map(|f| f.to_string()),
            file_type: ftype,
            content_type: Some(mime.to_string()),
            file_size: filesize,
            access_url: down_url,
            data: Value::Object(fields.to_owned()),
            copied: false,
        }
    } else {
        UploadFileInfo {
            file_id: Some(file_id),
            source: Some(source.to_string()),
            dest_file: Some(dest_name),
            dest_path: dest_file.to_str().map(|f| f.to_string()),
            file_type: ftype,
            content_type: Some(mime.to_string()),
            file_size: filesize,
            access_url: down_url,
            data: Value::Object(fields.to_owned()),
            copied: true,
        }
    }
}
