use anyhow::anyhow;
use chimes_store_core::dbs::{get_sys_path, PythonEnvConfig};
use chimes_store_core::service::invoker::InvocationContext;
use pyo3::{prelude::*, IntoPyObjectExt};
use pyo3::types::{IntoPyDict, PyBool, PyDict, PyFloat, PyInt, PyList, PyNone, PyString};
use serde_json::Value;
use std::ffi::CStr;
use std::sync::{Arc, Mutex};

use crate::growth::features::{required, py_get_namespace_config, py_get_plugin_config, PyInvocationContext, PythonStoreObject, PythonStorePlugin, PythonStoreQuery};

mod growth;



#[pymodule]
#[pyo3(name="growth")]
fn growth_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
  m.add_function(wrap_pyfunction!(required, m)?)?;
  m.add_function(wrap_pyfunction!(py_get_namespace_config, m)?)?;
  m.add_function(wrap_pyfunction!(py_get_plugin_config, m)?)?;
  m.add_class::<PythonStoreObject>()?;
  m.add_class::<PythonStoreQuery>()?;
  m.add_class::<PythonStorePlugin>()?;
  m.add_class::<PyInvocationContext>()?;
  Ok(())
}


#[no_mangle]
pub fn execute(
    ctx: Arc<Mutex<InvocationContext>>,
    script: &str,
    args: Vec<Value>,
    options: Option<Value>,
) -> Result<Option<Value>, anyhow::Error> {
    let conf = match options {
        Some(t) => serde_json::from_value::<PythonEnvConfig>(t).unwrap_or_default(),
        None => PythonEnvConfig::default(),
    };

    unsafe {
        if let Some(path) = get_sys_path() {
            let injectpath = conf.python_path.unwrap_or_default();
            #[cfg(target_os = "windows")]
            let newpath = format!("{injectpath};{path}");

            #[cfg(not(target_os = "windows"))]
            let newpath = format!("{}:{}", injectpath, path);

            // println!("Inject to new path env. {newpath}");
            std::env::set_var("PATH", newpath);
        }

        let workpath =  conf.workspace.unwrap_or(".".to_string());

        if let Some(sitepath) = conf.site_package.clone() {
            #[cfg(not(target_os = "windows"))]
            let fullsite = format!("{sitepath}:{workpath}");

            #[cfg(target_os = "windows")]
            let fullsite = format!("{sitepath};{workpath}");

            std::env::set_var("PYTHONPATH", fullsite);
        }
    }

    let mut nultrm_script = script.to_string();
    nultrm_script.push('\0');
    let return_item = conf.return_item.unwrap_or("ret".to_string());
    
    Python::attach(|py| {
        if let Some(sitepath) = conf.site_package {
            let site = py.import("site")?;
            let _ = site.call_method1("addsitedir", (sitepath,))?;
        }

        let pyctx = PyInvocationContext {
            ctx: ctx
        }.into_py_any(py)?.into_bound(py);

        let m = PyModule::new(py, "growth")?;
        growth_module(&m)?;
        let f = wrap_pyfunction!(required, &m)?;

        let g = [("growth", m.into_any()), ("required", f.into_any())].into_py_dict(py)?;
        
        let json_mod = py.import("json")?;
        let json_text = serde_json::to_string(&Value::Array(args))?;
        let py_obj = json_text.into_pyobject(py)?;
        let jc = json_mod.call_method1("loads", &(py_obj,))?;
        let locals = [("args", jc), ("ctx", pyctx)].into_py_dict(py)?;
        // let markitdown = [("markitdown", py.import("markitdown")?)].into_py_dict(py)?;
        let evcode = CStr::from_bytes_with_nul(nultrm_script.as_bytes())?; // c_str!(code.as_str());


        py.run(evcode, Some(&g), Some(&locals))?;
        // let content: String = result.getattr("text_content")?.extract()?;
        let result = match locals.get_item(&return_item)? {
            Some(t) => t,
            None => py.None().into_bound(py),
        };

        let res_obj = if result.is_instance_of::<PyDict>() {
            result.cast::<PyDict>().map(|t| t.as_any())
        } else if result.is_instance_of::<PyList>() {
            result.cast::<PyList>().map(|t| t.as_any())
        } else if result.is_instance_of::<PyString>() {
            result.cast::<PyString>().map(|t| t.as_any())
        } else if result.is_instance_of::<PyFloat>() {
            result.cast::<PyFloat>().map(|t| t.as_any())
        } else if result.is_instance_of::<PyInt>() {
            result.cast::<PyInt>().map(|t| t.as_any())
        } else if result.is_instance_of::<PyBool>() {
            result.cast::<PyBool>().map(|t| t.as_any())
        } else if result.is_none() {
            result.cast::<PyNone>().map(|t| t.as_any())
        } else {
            result.cast::<PyAny>().map(|t| t.as_any())
        };

        let obj = match res_obj {
            Ok(tt) => tt,
            Err(err) => {
                return Err(anyhow!("ERR: {err}"));
            }
        };

        let json = json_mod.call_method1("dumps", (obj,))?.to_string();

        serde_json::from_str(&json)
            .map(|t| Ok(Some(t)))
            .unwrap_or(Ok(None))
    })
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use crate::execute;
    use chimes_store_core::service::invoker::InvocationContext;
    use serde_json::json;

    #[test]
    pub fn test_execute_script() -> Result<(), anyhow::Error> {
        let opt = crate::PythonEnvConfig {
            python_path: Some(r#"E:\mishmash\conda\envs\pytorch"#.to_owned()),
            site_package: Some(r#"E:\mishmash\conda\envs\pytorch\Lib\site-packages"#.to_owned()),
            return_item: Some("ret".to_owned()),
            workspace: Some(".".to_owned())
        };
        let script = "import os\nimport json\nprint(args)\nobj=required(\"object://com.enjoylost.test/RagTestInfo\")\nprint(obj.get_invoke_uri())\nobj.invoke_return_option(\"find_one\", ctx, [{'a': 1, 'b': 2}])\nret = [ { 'a' : 1, 'b' : 2, 'c' : 3, 'd' : 4, 'e' : 5, 'x': obj.get_invoke_uri() } ]\n";
        let ctx = Arc::new(Mutex::new(InvocationContext::new()));
        let result = execute(
            ctx,
            script,
            vec![json!({"world": "System is the first way."})],
            Some(json!(opt)),
        )?;
        println!("Result: {result:?}");
        Ok(())
    }
}
