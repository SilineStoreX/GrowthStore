use std::{collections::HashMap, str::FromStr};

use anyhow::anyhow;
use serde_json::Value;
use tera::{Context, Tera};
use crate::algorithm::{sha1_256_hash, sha2_256_hash, md5_hash};

/**
 * function to impl hmac_sha1
 */
pub fn hmac_sha1(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = sha1_256_hash(&data.as_bytes());
        Ok(Value::String(ret))        
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn text_md5(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = md5_hash(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl hmac_sha1
 */
pub fn hmac_sha2(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = sha2_256_hash(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }
}

/**
 * function to impl base64_encode
 */
pub fn text_base64_encode(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let ret = crate::algorithm::base64_encode(&data.as_bytes());
        Ok(Value::String(ret))
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }    
}

/**
 * function to impl base64_decode
 */
pub fn text_base64_decode(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(Value::String(data)) = args.get("text") {
        let decode_data = crate::algorithm::base64_decode(&data.as_bytes());
        if let Ok(ret) = String::from_utf8(decode_data) {
            Ok(Value::String(ret))
        } else {
            Ok(Value::Null)
        }
    } else {
        Err(tera::Error::msg("No text arg provided"))
    }    
}

pub fn json_path(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    log::info!("args: {:?}", args);
    if let Some(arg) = args.get("arg") {
        if let Some(Value::String(path)) = args.get("path") {
            if let Some(t) = json_path_get(arg, path) {
                return Ok(t);
            }
        } else {
            return Err(tera::Error::msg("No path provided"));
        }
    } else {
        return Err(tera::Error::msg("No arg provided"));
    }
    Ok(Value::Null)
}

pub fn to_json(args: &HashMap<String, Value>) -> Result<Value, tera::Error> {
    if let Some(arg) = args.get("value") {
        match serde_json::to_string_pretty(arg) {
            Ok(t) => Ok(Value::String(t)),
            Err(err) => Err(tera::Error::msg(err)),
        }
    } else {
        Err(tera::Error::msg("No value provided"))
    }
}

pub fn template_eval(script: &str, ctx: Value) -> Result<String, anyhow::Error> {
    let mut tera = match Tera::new("templates/**/*.tera") {
        Ok(t) => t,
        Err(err) => {
            log::info!(
                "Could not found tera context {}, then use default Tera.",
                err
            );
            // return Err(ChimesError::custom(310, err.to_string()));
            Tera::default()
        }
    };

    let context = match Context::from_serialize(ctx) {
        Ok(c) => c,
        Err(_) => Context::new(),
    };

    tera.register_function("hmac_sha1", hmac_sha1);
    tera.register_function("hmac_sha2", hmac_sha2);
    tera.register_function("md5string", text_md5);
    tera.register_function("base64_encode", text_base64_encode);
    tera.register_function("base64_decode", text_base64_decode);

    tera.register_function("jsonpath", json_path);
    tera.register_function("to_json", to_json);
    match tera.render_str(script, &context) {
        Ok(text) => Ok(text),
        Err(err) => {
            log::info!("err: {err:?}");
            Err(anyhow!(err.to_string()))
        }
    }
}

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
