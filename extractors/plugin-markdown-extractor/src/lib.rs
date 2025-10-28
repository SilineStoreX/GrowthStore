use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ffi::CStr;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MarkdownExtractorConfig {
    pub python_path: Option<String>,
    pub site_package: Option<String>,
}

#[no_mangle]
pub fn extract_file(
    filename: &str,
    options: Option<Value>,
) -> Result<Option<String>, anyhow::Error> {
    let conf = match options {
        Some(t) => serde_json::from_value::<MarkdownExtractorConfig>(t).unwrap_or_default(),
        None => MarkdownExtractorConfig::default(),
    };

    unsafe {
        if let Ok(path) = std::env::var("PATH") {
            let injectpath = conf.python_path.unwrap_or_default();
            #[cfg(target_os = "windows")]
            let newpath = format!("{injectpath};{path}");

            #[cfg(not(target_os = "windows"))]
            let newpath = format!("{}:{}", injectpath, path);

            println!("Inject to new path env. {newpath}");
            std::env::set_var("PATH", newpath);
        }

        if let Some(sitepath) = conf.site_package.clone() {
            std::env::set_var("PYTHONPATH", sitepath);
        }
    }

    println!("Extract filename: {filename}");
    let refine_filename = filename.replace("\\", "/");
    Python::attach(|py| {
        if let Some(sitepath) = conf.site_package {
            let site = py.import("site")?;
            let _ = site.call_method1("addsitedir", (sitepath,))?;
        }

        let locals = [("os", py.import("os")?)].into_py_dict(py)?;
        let markitdown = [("markitdown", py.import("markitdown")?)].into_py_dict(py)?;
        let code =
            format!("markitdown.MarkItDown(enable_plugins=True).convert('{refine_filename}')\0");
        println!("eval code: {code}");
        let evcode = CStr::from_bytes_with_nul(code.as_bytes())?; // c_str!(code.as_str());

        let result = py.eval(evcode, Some(&markitdown), Some(&locals))?;
        let content: String = result.getattr("text_content")?.extract()?;
        Ok(Some(content))
    })
}
