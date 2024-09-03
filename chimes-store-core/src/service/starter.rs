use anyhow::Error;
use async_std::path::PathBuf;
use base64::Engine;
use futures_lite::Future;
use itertools::Itertools;
use rbatis::{Page, RBatis};
use serde_json::Value;
use std::{
    collections::HashMap, fs::{self, create_dir_all, remove_file, File}, io::{BufReader, Read, Write}, 
    mem::MaybeUninit, ops::Deref, path::Path, pin::Pin, sync::{Arc, Mutex, Once}
};

use crate::{
    config::{
        auth::JwtUserClaims, HookInvoker, MethodHook, PluginConfig, QueryObject, StoreObject,
        StoreServiceConfig,
    },
    utils::{build_path, build_path_ns, copy_to_slice, get_multiple_rbatis},
};

use super::{
    files::FileStoreManager,
    invoker::InvocationContext,
    registry::SchemaRegistry,
    sdk::{InvokeUri, RxHookInvoker, RxPluginService},
};

pub type FnPluginInstall = fn(ns: &str, plc: &PluginConfig) -> Result<(), anyhow::Error>;

#[repr(transparent)]
pub struct MxStoreService(pub(crate) StoreServiceConfig);

unsafe impl Send for MxStoreService {}
unsafe impl Sync for MxStoreService {}

impl MxStoreService {
    /**
     * 获取所有的
     */
    pub fn get_full_resources() -> Vec<String> {
        let nss = Self::get_namespaces();
        let mut urls = vec![];
        nss.into_iter().for_each(|ns| {
            if let Some(mxs) = Self::get(&ns) {
                mxs.get_objects().into_iter().for_each(|o| {
                    urls.push(format!("object://{}/{}", &ns, &o.name));
                });
                mxs.get_querys().into_iter().for_each(|o| {
                    urls.push(format!("query://{}/{}", &ns, &o.name));
                });
                mxs.get_plugins().into_iter().for_each(|o| {
                    urls.push(format!("{}://{}/{}", &o.protocol, &ns, &o.name));
                });
            }
        });
        urls
    }

    pub fn get_namespaces() -> Vec<String> {
        Self::get_store_service_map()
            .keys()
            .map(|f| f.to_owned())
            .collect_vec()
    }

    pub fn remove_namespace(ns: &str) {
        if let Some(mx) = Self::get(ns) {
            let pls = mx.get_plugins();
            let todelpls = pls.into_iter().map(|f| f.name).collect_vec();
            Self::update_service_delete_plugin(ns, &todelpls);
            if let Ok(path) = build_path(Self::get_model_path(), ns) {
                Self::delete_folder(path);
            }
            Self::delete_file(Self::get_model_path(), &mx.0.filename);
            Self::get_store_service_map().remove(ns);
        }

        
    }

    pub fn get_namespace(&self) -> String {
        self.0.namespace.clone()
    }

    pub fn get_config(&self) -> StoreServiceConfig {
        self.0.clone()
    }

    pub fn get(ns: &str) -> Option<&'static MxStoreService> {
        Self::get_store_service_map().get(ns)
    }

    pub fn remove_object(&mut self, name: &str) {
        let mut objmap = self.0.object_map.deref().to_owned();
        objmap.remove(name);
        self.0.object_map = Arc::new(objmap);
    }

    pub fn insert_object(&mut self, st: StoreObject) {
        let mut objmap = self.0.object_map.deref().to_owned();
        objmap.insert(st.name.clone(), st);
        self.0.object_map = Arc::new(objmap);
    }

    pub fn remove_query(&mut self, name: &str) {
        let mut objmap = self.0.query_map.deref().to_owned();
        objmap.remove(name);
        self.0.query_map = Arc::new(objmap);
    }

    pub fn insert_query(&mut self, st: QueryObject) {
        let mut objmap = self.0.query_map.deref().to_owned();
        objmap.insert(st.name.clone(), st);
        self.0.query_map = Arc::new(objmap);
    }

    pub fn remove_plugin(&mut self, name: &str) -> Option<PluginConfig> {
        let mut objmap = self.0.plugin_map.deref().to_owned();
        let rm = objmap.remove(name);
        self.0.plugin_map = Arc::new(objmap);
        rm
    }

    pub fn insert_plugin(&mut self, st: PluginConfig) {
        let mut objmap = self.0.plugin_map.deref().to_owned();
        objmap.insert(st.name.clone(), st);
        self.0.plugin_map = Arc::new(objmap);
    }

    pub(crate) fn get_plugin_installer() -> &'static mut Option<Box<FnPluginInstall>> {
        // 使用MaybeUninit延迟初始化
        static mut PLUGIN_INSTALLER_MAP: MaybeUninit<Option<Box<FnPluginInstall>>> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static INSTALLER_ONCE: Once = Once::new();

        INSTALLER_ONCE.call_once(|| unsafe {
            PLUGIN_INSTALLER_MAP.as_mut_ptr().write(None);
        });

        unsafe { &mut (*PLUGIN_INSTALLER_MAP.as_mut_ptr()) }
    }

    pub fn set_plugin_installer(func: FnPluginInstall) {
        Self::get_plugin_installer().replace(Box::new(func));
    }

    fn get_store_service_map() -> &'static mut HashMap<String, MxStoreService> {
        // 使用MaybeUninit延迟初始化
        static mut SERVICE_MAP: MaybeUninit<HashMap<String, MxStoreService>> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static STS_ONCE: Once = Once::new();

        STS_ONCE.call_once(|| unsafe {
            SERVICE_MAP.as_mut_ptr().write(HashMap::new());
        });

        unsafe { &mut (*SERVICE_MAP.as_mut_ptr()) }
    }

    fn get_store_service_metadata() -> &'static mut HashMap<String, String> {
        // 使用MaybeUninit延迟初始化
        static mut METADATA_MAP: MaybeUninit<HashMap<String, String>> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static META_ONCE: Once = Once::new();

        META_ONCE.call_once(|| unsafe {
            METADATA_MAP.as_mut_ptr().write(HashMap::new());
        });

        unsafe { &mut (*METADATA_MAP.as_mut_ptr()) }
    }

    pub fn set_metadata(key: &str, val: &str) {
        Self::get_store_service_metadata().insert(key.to_owned(), val.to_owned());
    }

    pub fn get_metadata(key: &str) -> Option<&String> {
        Self::get_store_service_metadata().get(key)
    }

    pub fn set_assets_path(path: &str) {
        Self::set_metadata("_assets_path", path);
    }

    pub fn set_config_path(path: &str) {
        Self::set_metadata("_config_path", path);
    }

    pub fn set_model_path(path: &str) {
        Self::set_metadata("_model_path", path);
    }

    pub fn get_model_path() -> String {
        Self::get_store_service_metadata()
            .get("_model_path")
            .map(|f| f.to_owned())
            .unwrap_or_else(|| {
                let crrpath = match std::env::current_exe() {
                    Ok(exefile) => match exefile.parent() {
                        Some(p) => p
                            .join("assets")
                            .join("models")
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        None => "".to_string(),
                    },
                    Err(_) => match std::env::current_dir() {
                        Ok(curpath) => curpath
                            .join("assets")
                            .join("models")
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        Err(_) => "".to_string(),
                    },
                };
                crrpath
            })
    }

    pub fn get_config_path() -> String {
        Self::get_store_service_metadata()
            .get("_config_path")
            .map(|f| f.to_owned())
            .unwrap_or_else(|| {
                let crrpath = match std::env::current_exe() {
                    Ok(exefile) => match exefile.parent() {
                        Some(p) => p
                            .join("assets")
                            .join("config")
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        None => "".to_string(),
                    },
                    Err(_) => match std::env::current_dir() {
                        Ok(curpath) => curpath
                            .join("assets")
                            .join("config")
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        Err(_) => "".to_string(),
                    },
                };
                crrpath
            })
    }

    pub fn get_assets_path() -> String {
        Self::get_store_service_metadata()
            .get("_assets_path")
            .map(|f| f.to_owned())
            .unwrap_or_else(|| {
                let crrpath = match std::env::current_exe() {
                    Ok(exefile) => match exefile.parent() {
                        Some(p) => p
                            .join("assets")
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        None => "".to_string(),
                    },
                    Err(_) => match std::env::current_dir() {
                        Ok(curpath) => curpath
                            .join("assets")
                            .canonicalize()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        Err(_) => "".to_string(),
                    },
                };
                crrpath
            })
    }

    fn get_plugin_service_map() -> &'static mut HashMap<String, Box<dyn RxPluginService>> {
        // 使用MaybeUninit延迟初始化
        static mut PLUGIN_SERVICE_MAP: MaybeUninit<HashMap<String, Box<dyn RxPluginService>>> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static PLUGIN_ONCE: Once = Once::new();

        PLUGIN_ONCE.call_once(|| unsafe {
            PLUGIN_SERVICE_MAP.as_mut_ptr().write(HashMap::new());
        });

        unsafe { &mut (*PLUGIN_SERVICE_MAP.as_mut_ptr()) }
    }

    pub fn get_plugin_service(ns: &str) -> Option<&dyn RxPluginService> {
        Self::get_plugin_service_map().get(ns).map(|f| f.as_ref())
    }

    pub fn update_service(ns: &str, sts: MxStoreService) {
        Self::get_store_service_map().insert(ns.to_owned(), sts);
    }

    pub fn register_plugin(ns: &str, pls: Box<dyn RxPluginService>) {
        Self::get_plugin_service_map().insert(ns.to_owned(), pls);
    }

    pub fn update_service_add_objects(ns: &str, sts: &[StoreObject]) {
        if let Some(fs) = Self::get_store_service_map().get_mut(ns) {
            for st in sts.iter().cloned() {
                // fs.0.object_map.insert(st.name.clone(), st);
                fs.insert_object(st);
            }
            fs.0.objects = fs.0.object_map.values().map(|f| f.to_owned()).collect();
        }
    }

    pub fn update_service_add_query(ns: &str, sts: &[QueryObject]) {
        if let Some(fs) = Self::get_store_service_map().get_mut(ns) {
            for st in sts.iter().cloned() {
                fs.insert_query(st);
            }
            fs.0.querys = fs.0.query_map.values().map(|f| f.to_owned()).collect();
        }
    }

    pub fn update_service_add_plugin(ns: &str, sts: &[PluginConfig]) {
        if let Some(fs) = Self::get_store_service_map().get_mut(ns) {
            for st in sts.iter().cloned() {
                // install the default Plugin config, we will generate a plugin config file directly
                log::debug!("Update Service add plugin: {ns}, {:?}", st);

                let mut mst = st.clone();

                let model_path = PathBuf::new().join(Self::get_model_path());
                if let Ok(build_config_path) =
                    build_path(model_path.clone().join(ns), st.config.clone())
                {
                    mst.config = build_config_path.to_string_lossy().to_string();
                    if let Some(installer_func) = Self::get_plugin_installer() {
                        if let Err(err) = installer_func(ns, &mst) {
                            log::warn!(
                                "Could not install  plugin {}, error is {:?}",
                                mst.name,
                                err
                            );
                        };
                        let nsuri = format!("{}://{}/{}", st.protocol, ns, st.name.clone());
                        if let Some(pls) = Self::get_plugin_service(&nsuri) {
                            if let Err(err) = pls.save_config(&mst) {
                                log::warn!(
                                    "Could not touch the config for {}, error is {:?}",
                                    mst.name,
                                    err
                                );
                            }
                        } else {
                            log::warn!("Plugin {} was not installed properly.", mst.name);
                        }
                    }
                } else {
                    log::warn!("Could not build the config path {}.", st.config.clone());
                }
                // add or replace the st
                //fs.0.plugin_map.insert(st.name.clone(), st);
                fs.insert_plugin(st);
            }

            fs.0.plugins = fs.0.plugin_map.values().map(|f| f.to_owned()).collect();
        }
    }

    pub fn update_service_delete_objects(ns: &str, sts: &[String]) {
        if let Some(fs) = Self::get_store_service_map().get_mut(ns) {
            for st in sts {
                fs.remove_object(st);
                // fs.0.object_map.borrow_mut().remove(st);
            }
            fs.0.objects = fs.0.object_map.values().map(|f| f.to_owned()).collect();
        }
    }

    pub fn update_service_delete_query(ns: &str, sts: &[String]) {
        if let Some(fs) = Self::get_store_service_map().get_mut(ns) {
            for st in sts {
                log::info!("{st} to be deleted.");
                fs.remove_query(st);
            }
            fs.0.querys = fs.0.query_map.values().map(|f| f.to_owned()).collect();
        }
    }

    pub fn update_service_delete_plugin(ns: &str, sts: &[String]) {
        if let Some(fs) = Self::get_store_service_map().get_mut(ns) {
            for st in sts {
                if let Some(pl) = fs.remove_plugin(st) {
                    if let Ok(conf_path) = build_path_ns(Self::get_model_path(), ns, pl.config.clone()) {
                        if let Err(err) = remove_file(conf_path) {
                            log::info!("Unable to remove the file {}, error {:?}.", pl.config, err);
                        }
                    } else {
                        log::debug!("Could not compose the plugin-config path.");
                    }
                }
            }
            log::info!("Current Plugin to delete {}", fs.0.plugins.len());
            log::info!("Current Plugin Maps {}", fs.0.plugin_map.len());
            fs.0.plugins = fs.0.plugin_map.values().map(|f| f.to_owned()).collect();
        }
    }

    fn delete_folder(path: impl AsRef<Path>) {
        let path_ = path.as_ref();
        if let Err(err) = fs::remove_dir(path_) {
            log::warn!("Could not remove the folder {path_:?}. {err:?}");
        }
    }

    fn delete_file(path: impl AsRef<Path>, filename: impl AsRef<Path>) {
        let path_ = path.as_ref();
        let filepath_ = path_.join(filename);
        if let Err(err) = fs::remove_file(&filepath_) {
            log::warn!("Could not remove the folder {filepath_:?}. {err:?}");
        }
    }    

    pub fn add_config(conf: &StoreServiceConfig) {
        let ns = conf.namespace.clone();
        let mut mutconf = conf.clone();
        mutconf.refine();
        Self::update_service(&ns, MxStoreService(mutconf));
    }

    pub fn store_service_foreach<F>(f: F)
    where
        F: FnMut(&MxStoreService),
    {
        Self::get_store_service_map().values().for_each(f);
    }

    pub fn load_all(path: impl AsRef<Path>) {
        // load all files in the stored path
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        Self::load_all(entry.path());
                    }
                    if meta.is_file() {
                        let filepath = entry.path();
                        if let Some(ext) = filepath.extension() {
                            let fileext = ext.to_string_lossy();
                            if fileext == "toml" {
                                Self::load(filepath);
                            }
                        }
                    }
                }
            }
        };
    }

    pub fn load(file: impl AsRef<Path>) {
        log::debug!("Load StoreService for {}", file.as_ref().to_string_lossy());
        match load_config::<StoreServiceConfig>(file) {
            Ok(conf) => {
                // conf.refine();
                Self::add_config(&conf);
            }
            Err(err) => {
                log::warn!("Could not load with error {:?}", err);
            }
        }
    }

    pub fn update_and_save_namespace(
        conf: &StoreServiceConfig,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<()> {
        let filepath = path.as_ref().join(conf.filename.clone());
        if let Some(parent) = filepath.parent() {
            if let Err(err) = create_dir_all(parent) {
                log::info!("Error for create dir: {}", err);
            }
        }
        save_config(&conf, filepath)?;
        Self::add_config(conf);
        Ok(())
    }

    pub fn save(namespace: &str, path: impl AsRef<Path>) -> anyhow::Result<()> {
        log::debug!(
            "Save StoreService configuration for {}",
            path.as_ref().to_string_lossy()
        );
        if let Some(sts) = Self::get(namespace) {
            let conf = sts.0.clone();
            let filepath = path.as_ref().join(conf.filename.clone());
            save_config(&conf, filepath)?;
        }
        Ok(())
    }

    pub fn save_all(path: impl AsRef<Path>) -> anyhow::Result<()> {
        // load all files in the stored path
        for entry in Self::get_store_service_map().values_mut() {
            let conf = entry.0.clone();
            let filepath = path.as_ref().join(conf.filename.clone());
            save_config(&conf, filepath)?;
        }
        Ok(())
    }

    pub fn get_objects(&self) -> Vec<StoreObject> {
        self.0.objects.clone()
    }

    pub fn get_querys(&self) -> Vec<QueryObject> {
        self.0.querys.clone()
    }

    pub fn get_plugins(&self) -> Vec<PluginConfig> {
        self.0.plugins.clone()
    }

    pub fn get_filestore(&self) -> FileStoreManager {
        FileStoreManager(self.0.clone())
    }

    pub fn get_rbatis(&self) -> &RBatis {
        get_multiple_rbatis(&self.0.db_url)
    }

    pub fn get_object(&self, name: &str) -> Option<StoreObject> {
        self.0.object_map.get(name).map(|f| f.to_owned())
    }

    pub fn get_query(&self, name: &str) -> Option<QueryObject> {
        self.0.query_map.get(name).map(|f| f.to_owned())
    }

    pub fn get_plugin_config(&self, name: &str) -> Option<PluginConfig> {
        self.0.plugin_map.get(name).map(|f| f.to_owned())
    }

    pub fn get_plugin_config_by_protocol(&self, protocol: &str) -> Vec<PluginConfig> {
        self.0
            .plugins
            .clone()
            .into_iter()
            .filter(|p| p.protocol == *protocol)
            .collect_vec()
    }

    pub fn aes_encode_text(&self, text: &str) -> String {
        if self.0.aes_solt.is_none() && self.0.aes_key.is_none() {
            text.to_owned()
        } else {
            let mut key = [0; 32];
            let mut iv = [0; 16];
            let solt = self.0.aes_solt.clone().unwrap_or_default();
            let key_text = self.0.aes_key.clone().unwrap_or_default();
            let key_ = key_text.as_bytes();
            let iv_ = solt.as_bytes();
            copy_to_slice(&mut iv, iv_);
            copy_to_slice(&mut key, key_);

            match crate::utils::crypto::aes256_cbc_encrypt(text.as_bytes(), &key, &iv) {
                Ok(ret) => base64::engine::general_purpose::STANDARD.encode(ret),
                Err(err) => {
                    log::debug!("aes error {:?}", err);
                    String::new()
                }
            }
        }
    }

    pub fn aes_decode_text(&self, text: &str) -> String {
        if self.0.aes_solt.is_none() && self.0.aes_key.is_none() {
            text.to_owned()
        } else {
            let mut key = [0; 32];
            let mut iv = [0; 16];
            let solt = self.0.aes_solt.clone().unwrap_or_default();
            let key_text = self.0.aes_key.clone().unwrap_or_default();
            let key_ = key_text.as_bytes();
            let iv_ = solt.as_bytes();
            copy_to_slice(&mut iv, iv_);
            copy_to_slice(&mut key, key_);

            match base64::engine::general_purpose::STANDARD.decode(text) {
                Ok(tt) => match crate::utils::crypto::aes256_cbc_decrypt(&tt, &key, &iv) {
                    Ok(ret) => String::from_utf8_lossy(&ret).to_string(),
                    Err(err) => {
                        log::debug!("aes error {:?}", err);
                        String::new()
                    }
                },
                Err(err) => {
                    log::debug!("base64 error {:?}", err);
                    String::new()
                }
            }
        }
    }

    /**
     * TODO: 实现直接调用Dbs的查询请求
     */
    pub async fn execute_query(ns: &str, ctx: Arc<Mutex<InvocationContext>>, query: &str, args: &[Value]) -> Result<Vec<Value>, anyhow::Error> {
        // let ctx =  Arc::new(Mutex::new(InvocationContext::new()));
        SchemaRegistry::get().invoke_direct_query(ns, ctx, query, args).await
    }

    pub async fn invoke_return_one(
        uri: String,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        SchemaRegistry::get()
            .invoke_return_option(&uri, ctx, &args)
            .await
    }

    pub async fn invoke_return_vec(
        uri: String,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, anyhow::Error> {
        SchemaRegistry::get()
            .invoke_return_vec(&uri, ctx, &args)
            .await
    }

    pub async fn invoke_return_page(
        uri: String,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Page<Value>, anyhow::Error> {
        SchemaRegistry::get()
            .invoke_return_page(&uri, ctx, &args)
            .await
    }

    pub fn check_rolebase_permission(
        uri: &InvokeUri,
        jwt: &JwtUserClaims,
        roles: &[String],
        bypass: bool,
    ) -> bool {
        if let Some(ms) = Self::get(&uri.namespace) {
            if uri.schema == *"object" {
                if let Some(mso) = ms.get_object(&uri.object) {
                    return mso.has_permission(uri, jwt, roles);
                }
            } else if uri.schema == *"query" {
                if let Some(mso) = ms.get_query(&uri.object) {
                    return mso.has_permission(uri, jwt, roles);
                }
            } else if uri.schema == *"redis" {
                return true;
            } else {
                // this is a plugin
                if let Some(pls) = MxStoreService::get_plugin_service(&uri.url_no_method()) {
                    return pls.has_permission(uri, jwt, roles, bypass);
                }
            }
        }

        false
    }
}

pub fn load_config<T>(path: impl AsRef<Path>) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(toml::from_str(&contents)?)
}

pub fn save_config<T>(conf: &T, path: impl AsRef<Path>) -> anyhow::Result<()>
where
    T: serde::ser::Serialize + ?Sized,
{
    log::debug!("Save Config: {}", path.as_ref().to_string_lossy());
    let mut file = File::create(path)?;
    let content = toml::to_string_pretty(conf)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

impl RxHookInvoker for MxStoreService {
    // Pre-hook: 前置Hook，在前置Hook中，主要是对参数进行预处理
    // Hook的操作是为了在具体的某个方法执行之前进行一定的处理，或者产生事件
    // 因此，我们需要对Hook的执行要有一定的约束
    // 为了更好的能够产生有效的行为，以及可以进行对应的操作，我们建议
    // Hook有两种，一个是Event，一种是需要返回值的
    // uri: 当前所执行的方法表示的URI
    // ctx: 执行上下文
    // args: 参数列表
    fn invoke_pre_hook_(
        uri: String,
        hooks: Vec<MethodHook>,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        let mut mut_args = args.clone();
        log::debug!("Invoke pre hook for {}", uri.clone());
        ctx.lock().unwrap().insert("HOOK_HANDLE_URI", uri);
        Box::pin(async move {
            for hook in hooks.iter() {
                if hook.before {
                    match hook.invoke_return_vec(ctx.clone(), mut_args.clone()).await {
                        Ok(ret_args) => {
                            if !hook.event && !ret_args.is_empty() {
                                mut_args = ret_args;
                            }
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
            }
            Ok(mut_args)
        })
    }

    async fn invoke_post_hook_(
        uri: String,
        hooks: Vec<MethodHook>,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, anyhow::Error> {
        let mut mut_args = args.clone();
        log::debug!("Invoke post hook for {}", uri.clone());
        ctx.lock().unwrap().insert("HOOK_HANDLE_URI", uri);
        for hook in hooks.iter() {
            if !hook.before {
                log::debug!("execute the post hook: {}", hook.script.clone());
                match hook.invoke_return_vec(ctx.clone(), mut_args.clone()).await {
                    Ok(ret_args) => {
                        if !hook.event && !ret_args.is_empty() {
                            mut_args = ret_args;
                        }
                    }
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        }
        Ok(mut_args)
    }
}
