use chimes_store_core::utils::{algorithm::{sha1_256_hash, sha2_256_hash, md5_hash}, crypto::{hmac_sha1, hmac_sha256, hmac_sha512}};
use rhai::Blob;

/**
 * function to impl hmac_sha1
 */
pub fn sha1_text(data: &str) -> rhai::Dynamic {
    let ret = sha1_256_hash(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl hmac_sha1
 */
pub fn text_md5(data: &str) -> rhai::Dynamic {
    let ret = md5_hash(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl hmac_sha1
 */
pub fn sha2_text(data: &str) -> rhai::Dynamic {
    let ret = sha2_256_hash(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_encode
 */
pub fn text_base64_encode(data: &str) -> rhai::Dynamic {
    let ret = chimes_store_core::utils::algorithm::base64_encode(&data.as_bytes());
    rhai::Dynamic::from(ret)
}

/**
 * function to impl base64_decode
 */
pub fn text_base64_decode(uri: &str) -> rhai::Dynamic {
    let decode_data = chimes_store_core::utils::algorithm::base64_decode(&uri.as_bytes());
    if let Ok(ret) = String::from_utf8(decode_data) {
        rhai::Dynamic::from(ret)
    } else {
        rhai::Dynamic::from(String::new())
    }
}


pub fn hmac_sha1_rhai(key: &str, data: &str) -> rhai::Dynamic {
    let codec = hmac_sha1(&key, &data);
    rhai::Dynamic::from_blob(codec)
}

pub fn hmac_sha256_rhai(key: &str, data: &str) -> rhai::Dynamic {
    let codec = hmac_sha256(&key, &data);
    rhai::Dynamic::from_blob(codec)
}

pub fn hmac_sha512_rhai(key: &str, data: &str) -> rhai::Dynamic {
    let codec = hmac_sha512(&key, &data);
    rhai::Dynamic::from_blob(codec)
}
