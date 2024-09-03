use std::str::FromStr;
use serde_json::Value;

pub fn json_path_get(t: &Value, path: &str) -> Option<Value> {
    let jspath = if path.starts_with("$.") {
        path.to_owned()
    } else {
        format!("$.{}", path)
    };

    if let Ok(inst) = jsonpath_rust::JsonPathInst::from_str(&jspath) {
        let slice = inst.find_slice(t);
        if slice.is_empty() {
            None
        } else if slice.len() == 1 {
            let ret = &slice[0].clone();
            Some(ret.to_owned())
        } else {
            let ret = Value::Array(slice.into_iter().map(|f| f.to_owned()).collect());
            Some(ret)
        }
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn json_path_get_string(t: &Value, path: &str) -> String {
    json_path_get(t, path).map(|f| {
        match f {
            Value::String(t) => t,
            _ => f.to_string()
        }
    }).unwrap_or_default()
}

