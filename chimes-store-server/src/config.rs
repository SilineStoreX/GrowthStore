use std::{
    mem::MaybeUninit, net::{IpAddr, Ipv4Addr}, path::PathBuf, sync::{Mutex, Once}
};

use derivative::Derivative;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LevelMapper {
    pub logger: String,
    pub level: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub web: WebConfig,
    pub listen: ListenerOption,
    pub managers: Vec<ManagerAccount>,
    pub app_keys: Vec<AppKey>,
    pub rsa_private_key: Option<String>,
    pub rsa_public_key: Option<String>,
    pub plugins: Vec<Plugin>,
    pub loggers: Vec<LevelMapper>,
    pub log_level: Option<String>,
    pub log_file: Option<String>,
    pub log_writemode: Option<String>,
    pub log_rotation: Option<String>,
    pub log_keepfiles: Option<u64>,
    pub log_console: Option<bool>,
    pub log_json: Option<bool>,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

#[derive(Debug, Clone, Derivative, Serialize, Deserialize)]
#[derivative(Default)]
#[serde(default)]
pub struct WebConfig {
    #[derivative(Default(value = "String::from(\"store-server\")"))]
    pub app_name: String,

    /// Path to the folder containing all models.
    #[derivative(Default(value = "String::from(\"assets/models\").into()"))]
    #[serde(skip_serializing)]
    pub model_path: PathBuf,

    #[derivative(Default(value = "String::from(\"assets/configs\").into()"))]
    #[serde(skip_serializing)]
    pub config_path: PathBuf,
    /// Name of the model.
    /// 
    #[derivative(Default(value = "String::from(\"utf-8\")"))]
    pub code_page: String,

    /// Tokio的工作线程数，当work_threads为0，为当前cpu cores的2倍
    #[derivative(Default(value = "0u16"))]
    pub work_threads: u16,

    /// 用于执行任务的工作线程数，当pool_size为0，为当前cpu cores的1倍
    #[derivative(Default(value = "0u16"))]
    pub pool_size: u16,
}

unsafe impl Send for WebConfig {}
unsafe impl Sync for WebConfig {}

#[derive(Debug, Derivative, Clone, Serialize, Deserialize)]
#[derivative(Default)]
pub struct ManagerAccount {
    pub username: String,
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub credentials: String,
}

unsafe impl Send for ManagerAccount {}
unsafe impl Sync for ManagerAccount {}

pub struct ManagerAccountConfig {
    pub managers: Vec<ManagerAccount>,
}

impl ManagerAccountConfig {
    pub fn get_mut() -> &'static mut Mutex<ManagerAccountConfig> {
        // 使用MaybeUninit延迟初始化
        static mut MA_CONF: MaybeUninit<Mutex<ManagerAccountConfig>> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static MA_ONCE: Once = Once::new();

        MA_ONCE.call_once(|| unsafe {
            MA_CONF.as_mut_ptr().write(Mutex::new(ManagerAccountConfig {
                managers: vec![]
            }));
        });
        unsafe { &mut *MA_CONF.as_mut_ptr() }
    }

    pub fn get_managers() -> Vec<ManagerAccount> {
        Self::get_mut().get_mut().unwrap().managers.clone()
    }

    pub fn update(managers: Vec<ManagerAccount>) {
        Self::get_mut().get_mut().unwrap().managers = managers;
    }
}

#[derive(Debug, Derivative, Clone, Serialize, Deserialize)]
#[derivative(Default)]
pub struct AppKey {
    pub app_id: String,
    pub secret_key: String,
}

unsafe impl Send for AppKey {}
unsafe impl Sync for AppKey {}

#[derive(Debug, Derivative, Clone, Serialize, Deserialize)]
#[derivative(Default)]
pub struct Plugin {
    pub protocol: String,
    pub plugin_dylib: String,
    pub logger: Option<String>,      // Log level for this plugin
    pub plugin_type: Option<String>, // Lang Extension or Function Extension, default is function
}

unsafe impl Send for Plugin {}
unsafe impl Sync for Plugin {}

#[derive(Debug, Derivative, Clone, Serialize, Deserialize)]
#[derivative(Default)]
pub struct ListenerOption {
    /// Ip to listen to.
    #[derivative(Default(value = "IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))"))]
    pub ip: IpAddr,

    /// 是否启动独立的管理端口，
    /// 如果该值为True，则会启用两个端口，一个端口（management_port）专门为管理员服务，另一个端口为API服务
    /// 如果该值为False，则只启用一个端口，同时提供管理员功能和普通的API功能
    #[derivative(Default(value = "Some(false)"))]
    pub using_management_port: Option<bool>,


    /// Binding Management port to serve /management/**.
    #[derivative(Default(value = "Some(17801u16)"))]
    pub management_port: Option<u16>,

    /// Binding port.
    #[derivative(Default(value = "17800u16"))]
    pub port: u16,
    /// Domain for certs. Certs are stored in `assets/certs/`.
    #[derivative(Default(value = "String::from(\"local\")"))]
    pub domain: String,
    /// Using acme to issue the certs if the domain is not local.
    #[derivative(Default(value = "false"))]
    pub acme: bool,
    /// Force to enable https. When acme is true, tls must be true.
    #[derivative(Default(value = "false"))]
    pub tls: bool,
    /// For JWT Token encoding and decoding.    
    #[derivative(Default(value = "String::from(\"storexs\")"))]
    pub slot: String,
    /// Whether the identifier is forced to pass even if JWT Token verification fails
    #[derivative(Default(value = "Some(false)"))]
    pub force_pass: Option<bool>,
    /// Token expiration time by second
    #[derivative(Default(value = "Some(86400u32)"))]
    pub expire_sec: Option<u32>,
}

unsafe impl Send for ListenerOption {}
unsafe impl Sync for ListenerOption {}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ThreadState {
    pub web_config: WebConfig,
}
