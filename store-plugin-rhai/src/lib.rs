use chimes_store_core::{
    config::PluginConfig,
    service::script::{ExtensionRegistry, LangExtensions},
};
use engine::{
    eval_file_return_one, eval_file_return_page, eval_file_return_vec, eval_script_return_one,
    eval_script_return_page, eval_script_return_vec, init_engin,
};

mod engine;

/**
 * Plugin开发手记
 * 无法使用tokio这样的runtime来在dylib与bin之间共享代码
 * 所以，会造成async fn无法准确的执行
 * 在Plugin中，无法使用主程序中定义的全局变量
 * 函数是一样的，但因为导出的方式不同  
 */

pub fn get_plugin_name() -> &'static str {
    "rhai"
}

/**
 * 初始化插件
 */
pub fn plugin_init(_ns: &str, conf: &PluginConfig) {
    log::info!(
        "Process the config of plugin and init the plugin for {}.",
        conf.name
    );
}

/**
 * Init the current extension
 * create the eval context of this lang script engine
 */
pub fn extension_init() {
    init_engin();
    let lang = LangExtensions::new("rhai", "RhaiScript")
        .with_return_option_script_fn(eval_script_return_one)
        .with_return_option_file_fn(eval_file_return_one)
        .with_return_vec_script_fn(eval_script_return_vec)
        .with_return_vec_file_fn(eval_file_return_vec)
        .with_return_page_script_fn(eval_script_return_page)
        .with_return_page_file_fn(eval_file_return_page);

    ExtensionRegistry::register("rhai", lang);
}
