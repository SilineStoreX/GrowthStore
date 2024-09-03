use anyhow::anyhow;
use itertools::Itertools;
use rbatis::Page;
use serde_json::Value;
use std::sync::Mutex;
use std::{
    collections::HashMap,
    mem::MaybeUninit,
    sync::{Arc, Once},
};

use super::invoker::InvocationContext;

pub type FnScriptReturnOptionEval = fn(
    script: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error>;
pub type FnScriptFileReturnOptionEval = fn(
    filename: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Option<Value>, anyhow::Error>;

pub type FnScriptReturnVecEval = fn(
    script: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error>;
pub type FnScriptFileReturnVecEval = fn(
    filename: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Vec<Value>, anyhow::Error>;

pub type FnScriptReturnPageEval = fn(
    script: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Page<Value>, anyhow::Error>;
pub type FnScriptFileReturnPageEval = fn(
    filename: &str,
    ctx: Arc<Mutex<InvocationContext>>,
    args: &[Value],
) -> Result<Page<Value>, anyhow::Error>;

#[derive(Default)]
pub struct LangExtensions {
    pub lang: String,
    pub full_name: String,
    pub fn_return_option_script: Option<FnScriptReturnOptionEval>,
    pub fn_return_option_file: Option<FnScriptFileReturnOptionEval>,
    pub fn_return_vec_script: Option<FnScriptReturnVecEval>,
    pub fn_return_vec_file: Option<FnScriptFileReturnVecEval>,
    pub fn_return_page_script: Option<FnScriptReturnPageEval>,
    pub fn_return_page_file: Option<FnScriptFileReturnPageEval>,
}

unsafe impl Send for LangExtensions {}

unsafe impl Sync for LangExtensions {}

impl LangExtensions {
    pub fn new(name: &str, desc: &str) -> Self {
        Self {
            lang: name.to_owned(),
            full_name: desc.to_string(),
            ..Default::default()
        }
    }

    pub fn with_return_option_script_fn(mut self, func: FnScriptReturnOptionEval) -> Self {
        self.fn_return_option_script = Some(func);
        self
    }

    pub fn with_return_option_file_fn(mut self, func: FnScriptFileReturnOptionEval) -> Self {
        self.fn_return_option_file = Some(func);
        self
    }

    pub fn with_return_vec_script_fn(mut self, func: FnScriptReturnVecEval) -> Self {
        self.fn_return_vec_script = Some(func);
        self
    }

    pub fn with_return_vec_file_fn(mut self, func: FnScriptFileReturnVecEval) -> Self {
        self.fn_return_vec_file = Some(func);
        self
    }

    pub fn with_return_page_script_fn(mut self, func: FnScriptReturnPageEval) -> Self {
        self.fn_return_page_script = Some(func);
        self
    }

    pub fn with_return_page_file_fn(mut self, func: FnScriptFileReturnPageEval) -> Self {
        self.fn_return_page_file = Some(func);
        self
    }
}

pub struct ExtensionRegistry {
    map: HashMap<String, Arc<LangExtensions>>,
}

impl ExtensionRegistry {
    pub fn get_mut() -> &'static mut ExtensionRegistry {
        // 使用MaybeUninit延迟初始化
        static mut EXTENSION_REG_MAP: MaybeUninit<ExtensionRegistry> = MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static EXTENSION_ONCE: Once = Once::new();

        EXTENSION_ONCE.call_once(|| unsafe {
            EXTENSION_REG_MAP.as_mut_ptr().write(ExtensionRegistry {
                map: HashMap::new(),
            });
        });

        unsafe { &mut (*EXTENSION_REG_MAP.as_mut_ptr()) }
    }

    pub fn get() -> &'static ExtensionRegistry {
        Self::get_mut()
    }

    pub fn get_extensions() -> Vec<(String, String)> {
        Self::get()
            .map
            .values()
            .map(|k| (k.lang.clone(), k.full_name.clone()))
            .collect_vec()
    }

    pub fn register(lang: &str, langext: LangExtensions) {
        Self::get_mut()
            .map
            .insert(lang.to_owned(), Arc::new(langext));
    }

    pub fn get_extension(lang: &str) -> Option<&Arc<LangExtensions>> {
        Self::get().map.get(lang)
    }

    pub async fn invoke_return_one(
        lang: &str,
        script: &str,
        ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        match Self::get_extension(lang) {
            Some(extension) => match extension.fn_return_option_script {
                Some(func) => func(script, ctx, &args),
                None => {
                    log::info!("no function implemented.");
                    Err(anyhow!("Not implemented"))
                }
            },
            None => {
                log::info!("no lang extension implemented.");
                Err(anyhow!("Not implemented"))
            }
        }
    }
}
