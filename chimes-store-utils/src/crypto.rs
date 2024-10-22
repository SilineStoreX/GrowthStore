use crypto::{
    aes::{self, KeySize},
    blockmodes::PkcsPadding,
    buffer::{BufferResult, ReadBuffer, RefReadBuffer, RefWriteBuffer, WriteBuffer},
    mac::Mac,
    symmetriccipher::SymmetricCipherError,
};

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

pub fn hmac_sha512(key: &str, data: &str) -> Vec<u8>  {
    use crypto::hmac::Hmac;
    use crypto::sha2::Sha512;
    let sha = Sha512::new();
    let mut hmac = Hmac::new(sha, key.as_bytes());
    hmac.input(data.as_bytes());
    let macres = hmac.result();
    macres.code().to_vec()
}
