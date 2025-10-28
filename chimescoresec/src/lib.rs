use std::sync::{Mutex, OnceLock};
use aes_wasm::aes256cbc::{IV_LEN, KEY_LEN};
use base64::{display::Base64Display, prelude::BASE64_STANDARD, Engine};
// use crypto::{aes::KeySize, blockmodes::PkcsPadding, buffer::{BufferResult, ReadBuffer, RefReadBuffer, RefWriteBuffer, WriteBuffer}, symmetriccipher::SymmetricCipherError};
use js_sys::{Promise, JSON};
use rsa::{pkcs8::DecodePublicKey, Pkcs1v15Encrypt, RsaPublicKey};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = fetch)]
    async fn fetch_with_js(input: &str, init: &JsValue) -> Promise;
}

#[derive(Serialize, Deserialize, Clone)]
struct RSAPrivateKey {
    pub private_key: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ApiResult<T> {
    pub status: Option<i64>,
    pub message: Option<String>,
    pub data: Option<T>,
    pub timestamp: Option<i64>,
}

static RSA_PRIVATE_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

#[wasm_bindgen]
pub async fn fetch_auth_config() -> Result<JsValue, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let url = format!("/api/auth/config");

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let json = JsFuture::from(resp.json()?).await?;

    log(&JSON::stringify(&json).unwrap().as_string().unwrap_or_default());

    // let jstr = JSON::stringify(&json).unwrap();
    if let Ok(result) = serde_wasm_bindgen::from_value::<ApiResult<RSAPrivateKey>>(json.clone()) {
        if let Some(rsapkey) = result.data {
            if let Some(key) = rsapkey.private_key {
                let privatekey = RSA_PRIVATE_KEY.get_or_init(|| Mutex::new(None));
                let rsa_ori_key = aes256_cbc_decrypt(&key);
                log(&rsa_ori_key);
                privatekey.lock().unwrap().replace(rsa_ori_key);
            } 
        }
    }

    // Send the JSON response back to JS.
    
    
    Ok(json)
}

fn aes256_cbc_decrypt(text: &str) -> String {
    let key = "growthxhpo9x32xdkllkcvlwkcdewkglqw";
    let solt = "r93lkxkddkksa3ex";
    let keyvec = key.as_bytes();
    let soltvec = solt.as_bytes();
    let mut keyuc = [0u8; KEY_LEN];
    let mut ivuc = [0u8; IV_LEN];

    copy_to_slice(&mut keyuc, keyvec);
    copy_to_slice(&mut ivuc, soltvec);

    if let Ok(decode) = base64::engine::general_purpose::STANDARD.decode(text) {
        if let Ok(res) = aes_wasm::aes256cbc::decrypt(&decode, &keyuc, ivuc) {
            String::from_utf8(res).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

pub fn copy_to_slice<T: Copy>(to: &mut [T], source: &[T]) {
    let to_len = to.len();
    to[..(to_len.min(source.len()))].copy_from_slice(&source[..(to_len.min(source.len()))]);
}

#[wasm_bindgen]
pub fn rsa_encrypt_text(token: &str) -> Option<String> {

    if let Some(st) = RSA_PRIVATE_KEY.get() {
        if let Some(key) = st.lock().unwrap().clone() {
            rsa_encrypt_with_public_key(token, &key)
        } else {
            None
        }
    } else {
        None
    }
}

#[wasm_bindgen]
pub fn rsa_encrypt_with_public_key(token: &str, public_key: &str) -> Option<String> {
    let bs = BASE64_STANDARD.decode(public_key).unwrap_or_default();

    let pub_key = match RsaPublicKey::from_public_key_der(&bs) {
        Ok(r) => Some(r),
        Err(_err) => {
            None
        }
    };

    match pub_key {
        Some(pkey) => {
            let mut rng = rsa::rand_core::OsRng::default();
            let encoded = pkey.encrypt(&mut rng, Pkcs1v15Encrypt, token.as_bytes());
            match encoded {
                Ok(rs) => {
                    let encodebase = Base64Display::new(&rs, &BASE64_STANDARD).to_string(); // .decode(rs);
                    Some(encodebase)
                }
                Err(_err) => {
                    None
                }
            }
        }
        None => None,
    }
}

