use chimes_store_utils::common::text_intern_trim;
use libloading::Library;
use serde_json::Value;

use super::filetypes::FileTypeMapping;

pub type FnExtractor = unsafe extern "Rust" fn(
    filename: &str,
    options: Option<Value>,
) -> Result<Option<String>, anyhow::Error>;

fn dummy_extract(
    _filename: &str,
    _options: Option<Value>,
) -> Result<Option<String>, anyhow::Error> {
    Ok(None)
}

#[allow(dead_code)]
pub struct FileExtrator {
    file_type: String,
    extractor_func: Box<FnExtractor>,
    library: Option<Library>,
    options: Option<Value>,
    dummy: bool,
}

impl FileExtrator {
    pub fn new(
        filetype: &str,
        extractor: FnExtractor,
        lib: Option<Library>,
        options: Option<Value>,
    ) -> Self {
        Self {
            file_type: filetype.to_owned(),
            extractor_func: Box::new(extractor),
            library: lib,
            options,
            dummy: false,
        }
    }

    pub fn new_dummy(filetype: &str) -> Self {
        Self {
            file_type: filetype.to_owned(),
            extractor_func: Box::new(dummy_extract),
            library: None,
            options: None,
            dummy: true,
        }
    }

    pub fn get_filetype(&self) -> String {
        self.file_type.clone()
    }

    pub fn extract(&self, filename: &str) -> Result<Option<String>, anyhow::Error> {
        let func = self.extractor_func.as_ref();
        unsafe {
            if let Some(t) = func(filename, self.options.clone())? {
                Ok(Some(text_intern_trim(&t)))
            } else {
                Ok(None)
            }
        }
    }
}

pub fn create_extractor(ftm: &FileTypeMapping) -> Option<FileExtrator> {
    let extr_name = ftm.factory.clone().unwrap_or_default().trim().to_owned();

    let (extractor_function, lib) = if let Some(libname) = ftm.library.clone() {
        if !libname.trim().is_empty() {
            unsafe {
                match libloading::Library::new(libname.clone()) {
                    Ok(tlib) => {
                        let func_extractor: Option<FnExtractor> =
                            match tlib.get(extr_name.as_bytes()) {
                                Ok(s) => Some(*s),
                                Err(err) => {
                                    log::error!("get the symbol failed {err}");
                                    None
                                }
                            };
                        (func_extractor, Some(tlib))
                    }
                    Err(err) => {
                        log::warn!("Could not load shared library {libname} by error {err}");
                        (None, None)
                    }
                }
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    if let Some(func) = extractor_function {
        Some(FileExtrator {
            file_type: ftm.ext_name.clone(),
            options: ftm.options.clone(),
            library: lib,
            extractor_func: Box::new(func),
            dummy: false,
        })
    } else {
        Some(FileExtrator::new_dummy(&ftm.ext_name.clone()))
    }
}
