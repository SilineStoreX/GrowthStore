use std::{collections::{hash_map::Entry, HashMap}, fmt::Debug, fs::File, io::Read, path::Path, sync::{Mutex, OnceLock}};

use crate::{common::copy_to_slice, crypto::{aes256_cbc_decrypt, aes256_cbc_encrypt}};
use anyhow::anyhow;
use base16ct::HexDisplay;
use base64::Engine;
use rbatis::snowflake::Snowflake;
use sha1::Digest;

pub fn md5_hash<T: AsRef<[u8]>>(data: &T) -> String {
    let hash = md5::compute(data);
    format!("{hash:x}")
}

pub fn to_hex<T: AsRef<[u8]>>(data: &T) -> String {
    let hex = HexDisplay(data.as_ref());
    format!("{hex:x}")
}

pub fn base32_encode<T: AsRef<[u8]>>(data: &T) -> String {
    base32::encode(base32::Alphabet::Z, data.as_ref())
}

pub fn base32_decode(data: &str) -> Vec<u8> {
    base32::decode(base32::Alphabet::Z, data).unwrap_or_else(|| {
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, data).unwrap_or_default()
    })
}

pub fn base64_encode<T: AsRef<[u8]>>(data: &T) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub fn base64_decode<T: AsRef<[u8]>>(data: &T) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .unwrap_or_default()
}

pub fn base64_encode_urlsafe<T: AsRef<[u8]>>(data: &T, urlsafe: bool) -> String {
    if urlsafe {
        base64::engine::general_purpose::URL_SAFE.encode(data)
    } else {
        base64::engine::general_purpose::STANDARD.encode(data)
    }
}

pub fn base64_decode_urlsafe<T: AsRef<[u8]>>(data: &T, urlsafe: bool) -> Vec<u8> {
    if urlsafe {
        base64::engine::general_purpose::URL_SAFE
            .decode(data)
            .unwrap_or_default()
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .unwrap_or_default()
    }
}

pub fn base64_encode_file<T: AsRef<Path> + Debug>(
    path: T,
    urlsafe: bool,
) -> Result<String, anyhow::Error> {
    match File::open(&path) {
        Ok(mut f) => {
            // let mut sha = sha1::Sha1::new();
            let mut buff = Vec::new();
            if let Err(err) = f.read_to_end(&mut buff) {
                return Err(anyhow!("read file error {err}"));
            }
            Ok(base64_encode_urlsafe(&buff, urlsafe))
        }
        Err(err) => Err(anyhow!(
            "error when calc the hash of the file {path:?} is error {err}"
        )),
    }
}

pub fn sha1_256_hash_file<T: AsRef<Path> + Debug>(path: T) -> Result<String, anyhow::Error> {
    match File::open(&path) {
        Ok(mut f) => {
            let mut sha = sha1::Sha1::new();
            let mut sbuf = [0u8; 16384];
            while let Ok(t) = f.read(&mut sbuf[..]) {
                if t > 0 {
                    sha.update(&sbuf[..t]);
                } else {
                    break;
                }
            }
            let hash = sha.finalize();
            let mut buf = [0u8; 64];
            match base16ct::lower::encode_str(&hash, &mut buf) {
                Ok(t) => Ok(t.to_owned()),
                Err(err) => Err(anyhow!("error when calc the file {path:?} hash {err}")),
            }
        }
        Err(err) => Err(anyhow!(
            "error when calc the hash of the file {path:?} is error {err}"
        )),
    }
}

pub fn sha2_256_hash_file<T: AsRef<Path>>(path: T) -> Result<String, anyhow::Error> {
    match File::open(path) {
        Ok(mut f) => {
            let mut sha = sha2::Sha256::new();
            let mut buf = [0u8; 16384];
            while let Ok(t) = f.read(&mut buf) {
                if t > 0 {
                    sha.update(&buf[..t]);
                } else {
                    break;
                }
            }
            let hash = sha.finalize();
            let mut buf = [0u8; 64];
            match base16ct::lower::encode_str(&hash, &mut buf) {
                Ok(t) => Ok(t.to_owned()),
                Err(err) => Err(anyhow!("error when calc the hash {err}")),
            }
        }
        Err(err) => Err(anyhow!("error when calc the hash {err}")),
    }
}

pub fn sha1_256_hash<T: AsRef<[u8]>>(data: &T) -> String {
    let mut sha = sha1::Sha1::new();
    sha.update(data);
    let hash = sha.finalize();
    let mut buf = [0u8; 64];
    match base16ct::lower::encode_str(&hash, &mut buf) {
        Ok(t) => t.to_owned(),
        Err(_) => String::new(),
    }
}

pub fn sha2_256_hash<T: AsRef<[u8]>>(data: &T) -> String {
    let hash = sha2::Sha256::digest(data);
    let mut buf = [0u8; 64];
    match base16ct::lower::encode_str(&hash, &mut buf) {
        Ok(t) => t.to_owned(),
        Err(_) => String::new(),
    }
}

pub fn aes_encrypt_text<T: AsRef<[u8]>>(data: &T, kstr: &str, ivstr: &str) -> String {
    let mut key = [0; 32];
    let mut iv = [0; 16];
    let kstr_bytes  = kstr.as_bytes();
    let ivstr_bytes = ivstr.as_bytes();

    copy_to_slice(&mut key, kstr_bytes);
    copy_to_slice(&mut iv, ivstr_bytes);
    
    aes_encrypt_to_text(data, &key, &iv)
}

pub fn aes_decrypt_text(data: &str, kstr: &str, ivstr: &str) -> String {
    let mut key = [0; 32];
    let mut iv = [0; 16];
    let kstr_bytes  = kstr.as_bytes();
    let ivstr_bytes = ivstr.as_bytes();

    copy_to_slice(&mut key, kstr_bytes);
    copy_to_slice(&mut iv, ivstr_bytes);
    
    aes_decrypt_to_text(data, &key, &iv)
}

pub fn aes_encrypt_to_text<T: AsRef<[u8]>>(data: &T, key: &[u8; 32], iv: &[u8; 16]) -> String {
    match aes256_cbc_encrypt(data.as_ref(), key, iv) {
        Ok(en) => base64_encode(&en),
        Err(_) => String::new(),
    }
}

pub fn aes_decrypt_to_text(data: &str, key: &[u8; 32], iv: &[u8; 16]) -> String {
    let decvec = base64_decode(&data.as_bytes());
    if decvec.is_empty() {
        return String::new();
    }

    match aes256_cbc_decrypt(&decvec, key, iv) {
        Ok(en) => String::from_utf8(en).unwrap_or_default(),
        Err(_) => String::new(),
    }
}

pub fn snowflake_id() -> i64 {
    rbatis::snowflake::new_snowflake_id()
}

pub static SNOWFLAKE_MAP: OnceLock<Mutex<HashMap<String, Snowflake>>> = OnceLock::new();

pub fn snowflake_id_custom(machine_id: i32, node_id: i32, mode: i32) -> i64 {
    let key = format!("{machine_id}-{node_id}-{mode}");

    let snowflake_map = SNOWFLAKE_MAP.get_or_init(|| Mutex::new(HashMap::new()));
    match snowflake_map.lock().unwrap().entry(key) {
        Entry::Occupied(cup) => {
            cup.get().generate()
        },
        Entry::Vacant(vac) => {
            let rflake = rbatis::snowflake::Snowflake::new(machine_id, node_id, mode);
            let newid = rflake.generate();
            vac.insert(rflake);
            newid
        }
    }
}

pub fn unwrap_json_i64(tval: &serde_json::Value, field_name: &str) -> i64 {
    match tval {
        serde_json::Value::Object(mp) => {
            if let Some(v) = mp.get(field_name) {
                match v {
                    serde_json::Value::Number(t) => t.as_i64().unwrap_or_default(),
                    serde_json::Value::String(t) => t.parse::<i64>().unwrap_or_default(),
                    _ => 0,
                }
            } else {
                0
            }
        }
        serde_json::Value::Number(t) => t.as_i64().unwrap_or_default(),
        serde_json::Value::String(t) => t.parse::<i64>().unwrap_or_default(),
        _ => 0,
    }
}

pub fn unwrap_json_f64(tval: &serde_json::Value, field_name: &str) -> f64 {
    match tval {
        serde_json::Value::Object(mp) => {
            if let Some(v) = mp.get(field_name) {
                match v {
                    serde_json::Value::Number(t) => t.as_f64().unwrap_or_default(),
                    serde_json::Value::String(t) => t.parse::<f64>().unwrap_or_default(),
                    _ => 0f64,
                }
            } else {
                0f64
            }
        }
        serde_json::Value::Number(t) => t.as_f64().unwrap_or_default(),
        serde_json::Value::String(t) => t.parse::<f64>().unwrap_or_default(),
        _ => 0f64,
    }
}

pub fn unwrap_json_bool(tval: &serde_json::Value, field_name: &str) -> bool {
    match tval {
        serde_json::Value::Object(mp) => {
            if let Some(v) = mp.get(field_name) {
                match v {
                    serde_json::Value::Bool(t) => t.to_owned(),
                    serde_json::Value::Number(t) => t.as_i64().unwrap_or_default() != 0,
                    serde_json::Value::String(t) => {
                        t.to_lowercase() == *"true" || t.to_lowercase() == *"yes"
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        serde_json::Value::Bool(t) => t.to_owned(),
        serde_json::Value::Number(t) => t.as_i64().unwrap_or_default() != 0,
        serde_json::Value::String(t) => t.to_lowercase() == *"true" || t.to_lowercase() == *"yes",
        _ => false,
    }
}

pub fn unwrap_json_string(tval: &serde_json::Value, field_name: &str) -> String {
    match tval {
        serde_json::Value::Object(mp) => {
            if let Some(v) = mp.get(field_name) {
                match v {
                    serde_json::Value::String(t) => t.clone(),
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        }

        serde_json::Value::String(t) => t.clone(),
        _ => String::new(),
    }
}
