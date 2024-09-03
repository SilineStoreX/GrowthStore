import JSEncrypt from 'jsencrypt/bin/jsencrypt.min'

// 密钥对生成 http://web.chacuo.net/netrsakeypair
const lowcase_chars = 'abcdefghijklmnopqrstuvwxyz0123456789_ABCDEFGHQTXXYZIJKLMN'
const publicKey = 'MFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBAKF+1MyYMe/jdl0Of0FgRzUhHv5ehJ1nAuzyNN6N6Gpd55QUSX0VukRpMPa3IP9odPF+I9pAChVJJfZeY339t18CAwEAAQ=='

// 加密
export function rsa_encrypt(txt: string) {
  const encryptor = new JSEncrypt()
  encryptor.setPublicKey(publicKey) // 设置公钥
  return encryptor.encrypt(txt) // 对需要加密的数据进行加密
}

export function aes_encrypt(txt: string, key: string, solt: string) {
    const encrypt = new JSEncrypt()
    return txt
}

export function md5_hash(txt: string) {
    return txt
}

export function sha1_hash(txt: string) {
    return txt
}

export function sha2_hash(txt: string) {
    return txt
}

function random_char() {
    let k = Math.ceil(Math.random() * lowcase_chars.length)
    if (k > 0 && k < lowcase_chars.length) {
        return lowcase_chars[k]
    } else {
        return lowcase_chars[0]        
    }
}

export function random_string(len: number) {
    var ts = '';
    for (var i = 0; i < len; i ++) {
        ts = ts + random_char()
    }
    return ts
}