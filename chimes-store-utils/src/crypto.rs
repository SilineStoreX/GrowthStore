use base64::{prelude::BASE64_STANDARD, Engine};
use crypto::{
    aes::{self, KeySize},
    blockmodes::PkcsPadding,
    buffer::{BufferResult, ReadBuffer, RefReadBuffer, RefWriteBuffer, WriteBuffer},
    mac::Mac,
    symmetriccipher::SymmetricCipherError,
};
use rsa::{pkcs8::DecodePrivateKey, Pkcs1v15Sign, RsaPrivateKey};
use sha2::Sha256;
// use rsa::signature::digest::Digest;
// use digest::Digest;
use crate::algorithm::base64_encode;
use digest::Digest;

/// Encrypt a buffer with the given key and iv using AES256/CBC/Pkcs encryption.
pub fn aes256_cbc_encrypt(
    data: &[u8],
    key: &[u8; 32],
    iv: &[u8; 16],
) -> Result<Vec<u8>, SymmetricCipherError> {
    let mut encryptor = aes::cbc_encryptor(KeySize::KeySize256, key, iv, PkcsPadding);

    let mut buffer = [0; 4096];
    let mut write_buffer = RefWriteBuffer::new(&mut buffer);
    let mut read_buffer = RefReadBuffer::new(data);
    let mut final_result = Vec::new();

    loop {
        let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true)?;
        final_result.extend(
            write_buffer
                .take_read_buffer()
                .take_remaining()
                .iter()
                .copied(),
        );
        match result {
            BufferResult::BufferUnderflow => break,
            _ => continue,
        }
    }

    Ok(final_result)
}

/// Encrypt a buffer with the given key using AES256/ECB/Pkcs encryption.
pub fn aes256_ebc_encrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, SymmetricCipherError> {
    let mut encryptor = aes::ecb_encryptor(KeySize::KeySize256, key, PkcsPadding);

    let mut buffer = [0; 4096];
    let mut write_buffer = RefWriteBuffer::new(&mut buffer);
    let mut read_buffer = RefReadBuffer::new(data);
    let mut final_result = Vec::new();

    loop {
        let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true)?;
        final_result.extend(
            write_buffer
                .take_read_buffer()
                .take_remaining()
                .iter()
                .copied(),
        );
        match result {
            BufferResult::BufferUnderflow => break,
            _ => continue,
        }
    }

    Ok(final_result)
}

/// Decrypt a buffer with the given key using AES256/EBC/Pkcs encryption.
pub fn aes256_ecb_decrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, SymmetricCipherError> {
    let mut decryptor = aes::ecb_decryptor(KeySize::KeySize256, key, PkcsPadding);

    let mut buffer = [0; 4096];
    let mut write_buffer = RefWriteBuffer::new(&mut buffer);
    let mut read_buffer = RefReadBuffer::new(data);
    let mut final_result = Vec::new();

    loop {
        let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true)?;
        final_result.extend(
            write_buffer
                .take_read_buffer()
                .take_remaining()
                .iter()
                .copied(),
        );
        match result {
            BufferResult::BufferUnderflow => break,
            _ => continue,
        }
    }

    Ok(final_result)
}

/// Decrypt a buffer with the given key and iv using AES256/CBC/Pkcs encryption.
pub fn aes256_cbc_decrypt(
    data: &[u8],
    key: &[u8; 32],
    iv: &[u8; 16],
) -> Result<Vec<u8>, SymmetricCipherError> {
    let mut decryptor = aes::cbc_decryptor(KeySize::KeySize256, key, iv, PkcsPadding);

    let mut buffer = [0; 4096];
    let mut write_buffer = RefWriteBuffer::new(&mut buffer);
    let mut read_buffer = RefReadBuffer::new(data);
    let mut final_result = Vec::new();

    loop {
        let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true)?;
        final_result.extend(
            write_buffer
                .take_read_buffer()
                .take_remaining()
                .iter()
                .copied(),
        );
        match result {
            BufferResult::BufferUnderflow => break,
            _ => continue,
        }
    }

    Ok(final_result)
}

pub fn hmac_sha1(key: &str, data: &str) -> Vec<u8> {
    use crypto::hmac::Hmac;
    use crypto::sha1::Sha1;
    let sha = Sha1::new();
    let mut hmac = Hmac::new(sha, key.as_bytes());
    hmac.input(data.as_bytes());
    let macres = hmac.result();
    macres.code().to_vec()
}

pub fn hmac_sha256(key: &str, data: &str) -> Vec<u8> {
    use crypto::hmac::Hmac;
    use crypto::sha2::Sha256;
    let sha = Sha256::new();
    let mut hmac = Hmac::new(sha, key.as_bytes());
    hmac.input(data.as_bytes());
    let macres = hmac.result();
    macres.code().to_vec()
}

pub fn hmac_sm3(key: &str, data: &str) -> Vec<u8> {
    use hmac_sm3::hmac_sm3;
    hmac_sm3(key.as_bytes(), data.as_bytes())
}

pub fn hex_encode_string(input: &[u8]) -> String {
    let elen = base16ct::encoded_len(input);
    let mut dst = vec![0u8; elen];
    let res = base16ct::lower::encode(input, &mut dst).expect("dst length is correct");
    debug_assert_eq!(elen, res.len());
    unsafe { String::from_utf8_unchecked(dst) }
}

pub fn hmac_sha512(key: &str, data: &str) -> Vec<u8> {
    use crypto::hmac::Hmac;
    use crypto::sha2::Sha512;
    let sha = Sha512::new();
    let mut hmac = Hmac::new(sha, key.as_bytes());
    hmac.input(data.as_bytes());
    let macres = hmac.result();
    macres.code().to_vec()
}

pub fn sha256_with_rsa_sign<T: AsRef<[u8]>>(data: &T, privkey: &str) -> String {
    let bs = BASE64_STANDARD.decode(privkey).unwrap_or_default();
    let priv_key = match RsaPrivateKey::from_pkcs8_der(&bs) {
        Ok(r) => Some(r),
        Err(err) => {
            tracing::warn!("Decode the Private Key with an error {err}");
            None
        }
    };

    match priv_key {
        Some(pkey) => {
            let mut hasher = Sha256::new();
            hasher.update(data.as_ref());
            let fix = hasher.finalize();
            match pkey.sign(Pkcs1v15Sign::new_unprefixed(), &fix) {
                Ok(rtvec) => base64_encode(&rtvec),
                Err(err) => {
                    tracing::error!("cound not sign this content {err}");
                    String::new()
                }
            }
        }
        None => String::new(),
    }
}
