use base64::Engine;
use crate::crypto::{aes256_cbc_decrypt, aes256_cbc_encrypt};
use sha2::Digest;

pub fn md5_hash<T: AsRef<[u8]>>(data: &T) -> String {
    let hash = md5::compute(data);
    format!("{:x}", hash)
}

pub fn base64_encode<T: AsRef<[u8]>>(data: &T) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub fn base64_decode<T: AsRef<[u8]>>(data: &T) -> Vec<u8> {
    match base64::engine::general_purpose::STANDARD.decode(data) {
        Ok(ts) => ts,
        Err(_) => {
            vec![]
        }
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

pub fn ase_encrypt_to_text<T: AsRef<[u8]>>(data: &T, key: &[u8; 32], iv: &[u8; 16]) -> String {
    match aes256_cbc_encrypt(data.as_ref(), key, iv) {
        Ok(en) => base64_encode(&en),
        Err(_) => String::new(),
    }
}

pub fn ase_decrypt_to_text<T: AsRef<[u8]>>(data: &str, key: &[u8; 32], iv: &[u8; 16]) -> String {
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

pub fn snowflake_id_custom(machine_id: i32, node_id: i32, mode: i32) -> i64 {
    rbatis::snowflake::Snowflake::new(machine_id, node_id, mode).generate()
}
