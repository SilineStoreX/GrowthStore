use std::{
    fs::File,
    io::{BufReader, Read, Write},
    mem::MaybeUninit,
    path::Path,
    sync::{Mutex, Once},
};

use crate::utils::{get_local_timestamp, global_data::i64_from_str};
use anyhow::Result;
use derivative::Derivative;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtUserClaims {
    pub username: String,
    pub userid: String,
    pub superadmin: bool,
    pub domain: String,  // use the for multi-organization system (SaaS etc)
    pub exp: i64,
}

unsafe impl Send for JwtUserClaims { }
unsafe impl Sync for JwtUserClaims { }

impl JwtUserClaims {
    pub fn anonymous() -> Self {
        JwtUserClaims {
            username: "anonymous".to_string(),
            userid: "0".to_owned(),
            superadmin: false,
            domain: "default".to_string(),
            exp: get_local_timestamp() as i64 + 30 * 60 * 1000,
        }
    }

    pub fn username(us: &str) -> Self {
        JwtUserClaims {
            username: us.to_owned(),
            userid: "0".to_owned(),
            superadmin: false,
            domain: "default".to_string(),
            exp: get_local_timestamp() as i64 + 30 * 60 * 1000,
        }
    }

    pub fn username_domain(us: &str, dm: &str) -> Self {
        JwtUserClaims {
            username: us.to_owned(),
            userid: "0".to_owned(),
            superadmin: false,
            domain: dm.to_owned(),
            exp: get_local_timestamp() as i64 + 30 * 60 * 1000,
        }
    }

    pub fn is_anonymous(&self) -> bool {
        self.username == *"anonymous"
    }
}

#[derive(Debug, Clone, Derivative, Serialize, Deserialize)]
#[derivative(Default)]
#[serde(default)]
pub struct AppSecretPair {
    pub app_id: String,
    pub app_secret: String,
    pub username: Option<String>,
    pub orgname: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Derivative, Serialize, Deserialize)]
#[derivative(Default)]
#[serde(default)]
pub struct AuthorizationConfig {
    pub enable: bool,                 // 启用Authorization服务
    pub validate_token_only: bool,    // 只开启Token的验证功能
    pub enable_captcha: bool,         // 图像码的验证
    pub validate_url: Option<String>, // 用于验证Authorization的URL，GET请求，返回用户信息。TODO: 产生HTTP请求
    pub validate_basic: bool, // 用于验证Authorization的URL只返回用户取基本的信息，可能只包含用户名(username)，状态(user_state)，锁定(user_locked),
    // 这时，如果需要获取用户详细信息和角色，需要根据用户username查询一次数据库。
    pub user_search: Option<String>, // URI表达，如：object://com.siline/User或如：query://com.siline/users
    pub role_search: Option<String>, // URI表达，如：object://com.siline/Role或如：query://com.siline/roles
    pub username_field: Option<String>,
    pub user_state_field: Option<String>,
    pub user_expire_field: Option<String>,
    pub user_credentials_field: Option<String>,
    pub user_lock_field: Option<String>,
    pub userid_field: Option<String>,
    pub reset_pwd_field: Option<String>,
    pub role_name_field: Option<String>,        // role_code ??
    pub role_name_presets: Option<String>,      // 预先定义的role_name表述
    pub credential_hash_method: Option<String>, // 密码加密的方法，md5, sha1, aes (可解密，由credential_solt来作为解密密码)
    pub credential_key: Option<String>, // 密码加密的盐，如果加密方法为RSA，则其值为Public Key
    pub credential_solt: Option<String>, // 密码加密的盐，如果加密方法为RSA，则其值为Private Key
    pub token_solt: Option<String>,     // jwt token生成时的盐
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub token_expire: Option<i64>, // jwt token的过期时长 default 30m

    pub fail_bypass: bool,
    /// Jwt Valdiation fail pass
    pub app_name: Option<String>,

    #[serde(default)]
    pub enable_organization: bool, // 启用组织机构校验

    pub organization_field: Option<String>, // 组织机构字段

    #[serde(default)]
    pub data_permission: bool, // 启用数据权限
    #[serde(default)]
    pub relative_table: Option<String>, // 数据权限相关联的表格
    #[serde(default)]
    pub permit_userfield: Option<String>, // 与用户ID相关联的字段
    pub permit_relative_field: Option<String>, // 数据权限相关联的字段
    #[serde(default)]
    pub enable_api_secure: bool,     // 启用APPID AppSecret对来交换获取Token
    #[serde(default)]
    pub check_relative_user: bool,
    pub appsecret_provider: Option<String>, // InvokeURI，用于查询AppSecretPair信息
    pub app_secret_keys: Vec<AppSecretPair>, // 有效的AppSecretPair对
}

unsafe impl Send for AuthorizationConfig {}
unsafe impl Sync for AuthorizationConfig {}

impl AuthorizationConfig {
    fn get_lock() -> &'static Mutex<AuthorizationConfig> {
        // 使用MaybeUninit延迟初始化
        static mut AUTH_CONF: MaybeUninit<Mutex<AuthorizationConfig>> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static AUTH_ONCE: Once = Once::new();

        AUTH_ONCE.call_once(|| unsafe {
            AUTH_CONF
                .as_mut_ptr()
                .write(Mutex::new(AuthorizationConfig {
                    ..Default::default()
                }));
        });
        unsafe { &*AUTH_CONF.as_ptr() }
    }

    pub fn get() -> AuthorizationConfig {
        Self::get_lock().lock().unwrap().clone()
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        log::debug!("Path: {:?}", path.as_ref());
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        reader.read_to_string(&mut contents)?;
        let authconf: Self = toml::from_str(&contents)?;
        if let Ok(mut locked) = Self::get_lock().lock() {
            *locked = authconf;
        }

        Ok(toml::from_str(&contents)?)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        log::debug!("Save Config: {}", path.as_ref().to_string_lossy());
        let mut file = File::create(path)?;
        let content = toml::to_string(self)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn get_role_name_presets(&self) -> Vec<String> {
        let mut roles = vec![
            "ROLE_SUPERADMIN".to_owned(),
            "ROLE_COMMONUSER".to_owned(),
            "ROLE_ANONYMOUS".to_owned(),
        ];

        if let Some(presets) = self.role_name_presets.clone() {
            presets.split(';').for_each(|f| {
                roles.push(f.trim().to_uppercase().to_string());
            })
        }
        roles.sort();
        roles.dedup();
        roles
    }
}
