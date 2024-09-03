use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, str::FromStr
};

use crate::{
    api,
    auth_service::AuthUserRole,
    config::{self, ListenerOption, ManagerAccountConfig, ThreadState},
    manager::{management_route, ManagementRequest, ManagementState},
    plugin::{load_plugin, static_load_plugin, PluginRegistry},
    utils::{AppConfig, PerformanceTaskCounter},
    Args,
};
use chimes_store_core::utils::{executor::CHIMES_THREAD_POOL, redis::init_global_redis, ApiResult};
use chimes_store_core::{
    config::{
        auth::{AuthorizationConfig, JwtUserClaims},
        PluginConfig,
    },
    service::{registry::SchemaRegistry, starter::MxStoreService},
    utils::build_path_ns
};
use chimes_store_dbs::api::get_management_redis_service_routers;
use chimes_store_dbs::api::get_salvo_service_router;
use salvo::{
    affix_state, catcher::Catcher, cors::{AllowHeaders, AllowOrigin, Cors}, fs::NamedFile, http::{Method, Mime}, jwt_auth::{ConstDecoder, HeaderFinder, QueryFinder}, logging::Logger, prelude::*, serve_static::StaticDir, Router
};
use serde::{Deserialize, Serialize};
use substring::Substring;
use clap::CommandFactory;
use salvo::conn::rustls::{Keycert, RustlsConfig};

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sid: String,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticServerPath {
    pub path: String,
}


fn plugin_router_install() -> Vec<Router> {
    let mut all_routers = vec![];
    PluginRegistry::get().install(&mut |pl| {
        if let Some(plugin_fun) = pl.fn_plugin_route_regiter {
            unsafe {
                let mut routers = plugin_fun();
                all_routers.append(&mut routers);
            }
        }
    });

    all_routers
}

fn plugin_anonymous_router_install() -> Vec<Router> {
    let mut all_routers = vec![];
    PluginRegistry::get().install(&mut |pl| {
        if let Some(plugin_fun) = pl.fn_plugin_anonymous_route_regiter {
            unsafe {
                let mut routers = plugin_fun();
                all_routers.append(&mut routers);
            }
        }
    });
    log::info!("anonymous request has {}", all_routers.len());
    all_routers
}

fn plugin_service_install(ns: &str, plc: &PluginConfig) -> Result<(), anyhow::Error> {
    PluginRegistry::get_mut().install(&mut |pl| {
        if let Ok(protocol) = pl.get_protocol_name() {
            if protocol == plc.protocol {
                if let Err(err) = pl.plugin_init(ns, plc) {
                    log::info!("Plugin {protocol} was not init. {}", err);
                }
            }
        }
    });
    Ok(())
}

fn plugin_extension_init() {
    PluginRegistry::get_mut().install(&mut |pl| {
        if let Err(err) = pl.extension_init() {
            log::info!("extension init failed {}", err);
        }
    });
}

#[handler]
async fn catcher_handler(res: &mut Response, ctrl: &mut FlowCtrl) {
    if let Some(stcode) = res.status_code {
        if stcode.as_u16() >= 400 {
            res.render(Json(ApiResult::<String>::error(
                stcode.as_u16() as i32,
                &stcode.to_string(),
            )));
            ctrl.skip_rest();
        }
    }
}

#[handler]
async fn send_text_file(res: &mut Response, req: &mut Request, depot: &mut Depot) {
    let serve_path: &StaticServerPath = depot.obtain::<StaticServerPath>().unwrap();
    let path: PathBuf = PathBuf::from_str(&serve_path.path.clone()).unwrap();
    let uripath = req.uri().path();
    let filepath = path.clone().join(uripath.substring(1, uripath.len()));
    // log::error!("Get the text file for {},  {filepath:?}", serve_path.path);
    NamedFile::builder(filepath).content_type(Mime::from_str("text/plain").unwrap()).send(req.headers(), res).await;
}

pub async fn salvo_main(args: Args, config: config::Config) {

    //let mut logger = simple_logger::SimpleLogger::new()
    //    .with_level(log::LevelFilter::Info)
    //    .with_module_level("store-server", log::LevelFilter::Info)
    //    .with_module_level("rbatis", log::LevelFilter::Info);

    
    let (sender, receiver) = flume::unbounded::<ManagementRequest>();
    tokio::task::spawn_blocking(move || management_route(receiver));

    let current_path = std::env::current_dir().unwrap();
    let listen = config.listen.clone();

    MxStoreService::set_assets_path(
        &current_path
            .clone()
            .join("assets")
            .canonicalize()
            .unwrap_or("assets/".to_owned().into())
            .to_string_lossy(),
    );
    MxStoreService::set_config_path(
        &config
            .web
            .config_path
            .canonicalize()
            .unwrap()
            .to_string_lossy(),
    );
    MxStoreService::set_model_path(
        &config
            .web
            .model_path
            .canonicalize()
            .unwrap()
            .to_string_lossy(),
    );


    //for l in config.loggers.clone() {
    //    if let Some(level) = l.level {
    //        if let Ok(lv) = log::LevelFilter::from_str(&level) {
    //            logger = logger.with_module_level(&l.logger, lv);
    //        } else {
    //            logger = logger.with_module_level(&l.logger, log::LevelFilter::Info);
    //        }
    //    } else {
    //        logger = logger.with_module_level(&l.logger, log::LevelFilter::Info);
    //    }
    //}

    MxStoreService::set_plugin_installer(plugin_service_install);

    // 注册所有的插件
    #[cfg(not(feature="plugin_rlib"))]
    config.plugins.clone().into_iter().for_each(|f| {
        if let Ok((lib, t)) = load_plugin(&f.plugin_dylib) {
            // add into this plugin registry
            //println!("Plugin for {} was loaded.", f.protocol);
            PluginRegistry::get_mut().register(lib, t);
        }
    });

    #[cfg(feature="plugin_rlib")]
    let _ = load_plugin("");

    static_load_plugin().iter().for_each(|f| {
        PluginRegistry::get_mut().register_static(f.to_owned());
    });

    ManagerAccountConfig::update(config.managers.clone());

    // logger.init().unwrap();

    AppConfig::update(&config.web);

    // let request = Box::new(config.clone().try_into().unwrap());
    if let Err(err) =
        AuthorizationConfig::load(config.web.config_path.clone().join("Authorization.toml"))
    {
        log::error!("Could not load authorization configuration. {}", err);
    }

    init_global_redis();

    let serve_path = {
        let current_path = std::env::current_dir().unwrap();

        //load_web("assets/www/index.zip", &path).expect("load frontend failed");
        current_path.join("assets/www/")
    };

    let mgr_path = {
        let current_path = std::env::current_dir().unwrap();

        //load_web("assets/www/index.zip", &path).expect("load frontend failed");
        current_path.join("assets/management/")
    };    

    log::info!("Prepared to load MxStoreService for all namespaces");
    MxStoreService::load_all(config.web.model_path.clone());
    // 为每一个Namespace启用插件
    PluginRegistry::get().install(&mut |rl| {
        let protocol_ = rl.get_protocol_name().unwrap_or_default();
        log::info!("start plugin {protocol_} ...");
        MxStoreService::get_namespaces().into_iter().for_each(|ns| {
            log::info!("init plugin {protocol_} for {}", ns);
            chimes_store_dbs::register_objects_and_querys(&ns);
            if let Some(nss) = MxStoreService::get(&ns) {
                if let Ok(protocol) = rl.get_protocol_name() {
                    let plcs = nss.get_plugin_config_by_protocol(&protocol);
                    for mut plc in plcs {
                        plc.config =
                            build_path_ns(config.web.model_path.clone(), &ns, plc.config.clone())
                                .unwrap_or(PathBuf::from(plc.config))
                                .to_string_lossy()
                                .to_string();
                        if let Err(err) = rl.plugin_init(&ns, &plc) {
                            log::info!("Plugin {protocol} was not init. {}", err);
                        }
                    }
                } else {
                    log::info!("The {ns} was got the plugin_init function.");
                }
            }
        });
    });

    plugin_extension_init();

    CHIMES_THREAD_POOL.setup_counter(Box::new(PerformanceTaskCounter()));

    let cors = Cors::new()
        .allow_origin(AllowOrigin::judge(|_, _, _| true))
        .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
        .into_handler();

    let authorization_solt = AuthorizationConfig::get()
        .token_solt
        .unwrap_or("AuthorizationJWTToken".to_string());

    let user_fail_bypass = AuthorizationConfig::get().fail_bypass;

    let auth_handler: JwtAuth<JwtUserClaims, _> =
        JwtAuth::new(ConstDecoder::from_secret(authorization_solt.as_bytes()))
            .finders(vec![
                Box::new(HeaderFinder::new()),
                Box::new(QueryFinder::new("_token")),
                // Box::new(CookieFinder::new("jwt_token")),
            ])
            .force_passed(user_fail_bypass);

    let mgr_auth_handler: JwtAuth<JwtUserClaims, _> =
            JwtAuth::new(ConstDecoder::from_secret(authorization_solt.as_bytes()))
                .finders(vec![
                    Box::new(HeaderFinder::new()),
                    Box::new(QueryFinder::new("_token")),
                    // Box::new(CookieFinder::new("jwt_token")),
                ])
                .force_passed(user_fail_bypass);

    let manager_auth_handler: JwtAuth<JwtClaims, _> =
        JwtAuth::new(ConstDecoder::from_secret(config.listen.slot.as_bytes()))
            .finders(vec![
                Box::new(HeaderFinder::new()),
                Box::new(QueryFinder::new("_token")),
                // Box::new(CookieFinder::new("jwt_token")),
            ])
            .force_passed(false);

    let api_router = Router::with_path("/api")
        .hoop(auth_handler)
        .hoop(AuthUserRole(true)) // should check the permission
        .push(Router::with_path("/auth/info").get(api::auth::user_auth_info))
        .push(Router::with_path("/auth/refresh").get(api::auth::user_auth_refresh))
        .push(Router::with_path("/auth/change_pwd").post(api::auth::user_auth_change_pwd))
        .push(Router::with_path("/auth/logout").get(api::auth::user_auth_logout))
        .push(Router::with_path("/execute/option").post(api::common::common_invoke_option))
        .push(Router::with_path("/execute/list").post(api::common::common_invoke_vec))
        .push(Router::with_path("/execute/paged").post(api::common::common_invoke_page))
        .push(Router::with_path("/execute/option").put(api::common::common_invoke_option))
        .push(Router::with_path("/execute/list").put(api::common::common_invoke_vec))
        .push(Router::with_path("/execute/paged").put(api::common::common_invoke_page))
        .push(Router::with_path("/file/<ns>/get/<file_id>").get(api::common::common_file_send))
        .append(&mut get_salvo_service_router())
        .append(&mut plugin_router_install());

    let manger_router = Router::with_path("/api")
        .hoop(mgr_auth_handler)
        .hoop(AuthUserRole(true)) // should check the permission
        .push(Router::with_path("/auth/info").get(api::auth::user_auth_info))
        .push(Router::with_path("/auth/refresh").get(api::auth::user_auth_refresh))
        .push(Router::with_path("/auth/change_pwd").post(api::auth::user_auth_change_pwd))
        .push(Router::with_path("/auth/logout").get(api::auth::user_auth_logout))
        .push(Router::with_path("/execute/option").post(api::common::common_invoke_option))
        .push(Router::with_path("/execute/list").post(api::common::common_invoke_vec))
        .push(Router::with_path("/execute/paged").post(api::common::common_invoke_page))
        .push(Router::with_path("/execute/option").put(api::common::common_invoke_option))
        .push(Router::with_path("/execute/list").put(api::common::common_invoke_vec))
        .push(Router::with_path("/execute/paged").put(api::common::common_invoke_page))
        .push(Router::with_path("/file/<ns>/get/<file_id>").get(api::common::common_file_send))
        .append(&mut get_salvo_service_router())
        .append(&mut plugin_router_install());    

    let app_router = Router::new()
        //.hoop(CorsLayer::permissive())
        .hoop(Logger::new())
        .hoop(cors.clone())
        .hoop(
            affix_state::inject(ThreadState {
                web_config: config.web.clone(),
            })
            .insert("config", config.clone()),
        )
        .push(
            Router::with_path("/api")
                .push(Router::with_path("auth/code_image").get(api::auth::user_auth_auth_code))
                .push(Router::with_path("auth/login").post(api::auth::user_auth_login))
                .push(Router::with_path("auth/exchange").post(api::auth::user_app_exchange))
                .push(Router::with_path("auth/exchange").get(api::auth::user_app_exchange))
                .push(
                    Router::with_path("metadata/<ns>/api-doc/openapi.json")
                        .get(api::management::metadata_openapi),
                )
                .push(
                    Router::with_path("metadata/<schema>/schema.json")
                        .get(api::management::metadata_get),
                )
                .push(
                    Router::with_path("metadata/<schema>/<ns>/<name>.json")
                        .get(api::management::metadata_get),
                )
                .push(
                    Router::with_path("passoff")
                        .hoop(AuthUserRole(false))
                        .append(&mut plugin_anonymous_router_install()),
                ),
        )
        .push(api_router);
    let cmd = Args::command();
    let version = cmd.get_version().unwrap_or("0.0.1");
    let bin_name = cmd.get_bin_name().unwrap_or("store-server");

    let doc = OpenApi::new(bin_name, version).merge_router(&app_router);
    let svpath = StaticServerPath { path: serve_path.clone().as_os_str().to_string_lossy().to_string() };
    let app = app_router
        .push(doc.into_router("/api-doc/openapi.json"))
        .push(SwaggerUi::new("/api-doc/openapi.json").into_router("swagger-ui"))
        .push(Router::with_path("/SERVICE-AGGREEMENT").hoop(affix_state::inject(svpath.clone())).get(send_text_file))
        .push(Router::with_path("/SERVICE-PERSONAL-PRIVACY").hoop(affix_state::inject(svpath.clone())).get(send_text_file))
        .push(Router::with_path("/SERVICE-LICENSE-APACHE").hoop(affix_state::inject(svpath.clone())).get(send_text_file))
        .push(Router::with_path("/LICENSE-APACHE").hoop(affix_state::inject(svpath.clone())).get(send_text_file))
        .push(Router::with_path("/PERSONAL-PRIVACY").hoop(affix_state::inject(svpath.clone())).get(send_text_file))
        .push(
            // this static serve should be after `swagger`
            Router::with_path("<**path>").get(StaticDir::new(serve_path).defaults(["index.html"])),
        );

    let mgr =  Router::new()
        //.hoop(CorsLayer::permissive())
        .hoop(Logger::new())
        .hoop(cors.clone())
        .hoop(
            affix_state::inject(ThreadState {
                web_config: config.web.clone(),
            })
            .insert("config", config.clone()),
        )
        .push(Router::with_path("/management/login").post(api::management::signin))
        .push(
            Router::with_path("/management")
                .hoop(affix_state::inject(ManagementState { sender }).insert("config", config.clone()))
                .hoop(manager_auth_handler)
                .push(Router::with_path("performance/get").get(api::performance::performance_get))
                .push(Router::with_path("userinfo").post(api::management::userinfo))
                .push(Router::with_path("changepwd").post(api::management::change_pwd))
                .push(Router::with_path("reload").post(api::management::reload))
                .push(Router::with_path("save").post(api::management::save))
                .push(Router::with_path("authorization").get(api::management::auth_conf))
                .push(Router::with_path("authorization").post(api::management::save_auth_conf))
                .push(Router::with_path("authorize/roles").get(api::management::auth_roles))
                .push(Router::with_path("update").post(api::management::update))
                .push(Router::with_path("delete").post(api::management::delete))
                .push(Router::with_path("probe/schema").get(api::management::probe_schema))
                .push(Router::with_path("probe/table").get(api::management::probe_table))
                .push(Router::with_path("generate").post(api::management::generate))
                .push(Router::with_path("fetch/namespaces").get(api::management::fetch_namespaces))
                .push(Router::with_path("fetch/pluginnames").get(api::management::fetch_plugin_name))
                .push(Router::with_path("plugin/list").get(api::management::plugin_list))
                .push(Router::with_path("lang/list").get(api::management::extension_list))
                .push(Router::with_path("config/get").get(api::management::config_get))
                .push(Router::with_path("config/save").post(api::management::config_save))
                .push(Router::with_path("config/archive").get(api::management::export_namespace))
                .push(Router::with_path("config/restore").post(api::management::restore_namespace))
                .push(Router::with_path("config/create").post(api::management::create_namespace))
                .push(
                    Router::with_path("metadata/generate").post(api::management::metadata_generate),
                )
                .push(
                    Router::with_path("metadata/<schema>/<ns>/<name>.json")
                        .get(api::management::metadata_get),
                )
                .push(
                    Router::with_path("metadata/<schema>/schema.json")
                        .get(api::management::metadata_get),
                )
                .push(Router::with_path("fetch/config").get(api::management::namespace_config))
                .push(Router::with_path("common/<protocol>/<ns>/<name>/<method>").post(api::management::execute_common_management))
                .append(&mut get_management_redis_service_routers()),
        )
        .push(
            Router::with_path("/api")
                .push(Router::with_path("auth/code_image").get(api::auth::user_auth_auth_code))
                .push(Router::with_path("auth/login").post(api::auth::user_auth_login))
                .push(Router::with_path("auth/exchange").post(api::auth::user_app_exchange))
                .push(Router::with_path("auth/exchange").get(api::auth::user_app_exchange))
                .push(
                    Router::with_path("metadata/<ns>/api-doc/openapi.json")
                        .get(api::management::metadata_openapi),
                )
                .push(
                    Router::with_path("metadata/<schema>/schema.json")
                        .get(api::management::metadata_get),
                )
                .push(
                    Router::with_path("metadata/<schema>/<ns>/<name>.json")
                        .get(api::management::metadata_get),
                )
                .push(
                    Router::with_path("passoff")
                        .hoop(AuthUserRole(false))
                        .append(&mut plugin_anonymous_router_install()),
                ),
        )
        .push(manger_router)
        .push(SwaggerUi::new("/api-doc/openapi.json").into_router("swagger-ui"))
        .push(Router::with_path("/SERVICE-AGGREEMENT").hoop(affix_state::inject(mgr_path.clone())).get(send_text_file))
        .push(Router::with_path("/SERVICE-PERSONAL-PRIVACY").hoop(affix_state::inject(mgr_path.clone())).get(send_text_file))
        .push(Router::with_path("/SERVICE-LICENSE-APACHE").hoop(affix_state::inject(mgr_path.clone())).get(send_text_file))
        .push(Router::with_path("/LICENSE-APACHE").hoop(affix_state::inject(mgr_path.clone())).get(send_text_file))
        .push(Router::with_path("/PERSONAL-PRIVACY").hoop(affix_state::inject(mgr_path.clone())).get(send_text_file))
        .push(
            // this static serve should be after `swagger`
            Router::with_path("<**path>").get(StaticDir::new(mgr_path).defaults(["index.html"])),
        );

    let service = Service::new(app)
        .catcher(Catcher::default().hoop(catcher_handler))
        .hoop(cors.clone());

    let mgr_service = Service::new(mgr)
        .catcher(Catcher::default().hoop(catcher_handler))
        .hoop(cors);

    let ip_addr = args.ip.unwrap_or(listen.ip);
    
    let port = args.port.unwrap_or(listen.port);
    let mgr_port = listen.management_port.unwrap_or(port + 1);
    
    let (acme, tls) = match listen.domain.as_str() {
        "local" => (false, listen.tls),
        _ => (listen.acme, true),
    };
    
    // let mgr_addr = SocketAddr::new(IpAddr::V4(ipv4_addr), mgr_port);
    if Some(true) == listen.using_management_port {
        tokio::try_join!(
            start_serve(acme, tls, listen.clone(), ip_addr, port, service),
            start_serve(acme, tls, listen, ip_addr, mgr_port, mgr_service),
        ).unwrap();
    } else {
        start_serve(acme, tls, listen, ip_addr, port, mgr_service).await.unwrap();
    }

}

async fn start_serve(acme: bool, tls: bool, listen: ListenerOption, ip_addr: IpAddr, port: u16, service: Service) -> Result<(), std::io::Error> {
    
    let (ipv4_addr, ipv6_addr) = match ip_addr {
        IpAddr::V4(addr) => (addr, None),
        IpAddr::V6(addr) => (Ipv4Addr::new(0, 0, 0, 0), Some(addr)),
    };

    let addr = SocketAddr::new(IpAddr::V4(ipv4_addr), port);

    if acme {
        let listener = TcpListener::new(addr)
            .acme()
            .cache_path("assets/certs")
            .add_domain(listen.domain.clone())
            .quinn(addr);

        if let Some(ipv6_addr) = ipv6_addr {
            if ipv6_addr.is_unspecified() && ipv4_addr.is_unspecified() {
                panic!("both IpV4 and IpV6 addresses are unspecified");
            }
            let addr_v6 = SocketAddr::new(IpAddr::V6(ipv6_addr), port);
            let acceptor = listener.join(TcpListener::new(addr_v6))
                                                                            .bind().await;
            
            log::warn!("server started at {addr} with acme and tls");
            log::warn!("server started at {addr_v6} with acme and tls");
            salvo::server::Server::new(acceptor).try_serve(service).await
        } else {
            let acceptor = listener.bind().await;
            log::warn!("server started at {addr} with acme and tls.");
            salvo::server::Server::new(acceptor).try_serve(service).await
        }
    } else if tls {
        let config = RustlsConfig::new(
            Keycert::new()
                .cert_from_path("assets/certs/cert.pem")
                .expect("unable to find cert.pem")
                .key_from_path("assets/certs/key.pem")
                .expect("unable to fine key.pem"),
        );
        let listener = TcpListener::new(addr).rustls(config.clone());
        if let Some(ipv6_addr) = ipv6_addr {
            let addr_v6 = SocketAddr::new(IpAddr::V6(ipv6_addr), port);
            let ipv6_listener = TcpListener::new(addr_v6)
                                                                                        .rustls(config.clone());
            #[cfg(not(target_os = "windows"))]
            let acceptor = QuinnListener::new(config.clone(), addr_v6)
                .join(ipv6_listener)
                .bind()
                .await;
            #[cfg(target_os = "windows")]
            let acceptor = QuinnListener::new(config.clone(), addr)
                .join(QuinnListener::new(config, addr_v6))
                .join(ipv6_listener)
                .join(listener)
                .bind()
                .await;
            log::warn!("server started at {addr} with tls");
            log::warn!("server started at {addr_v6} with tls");
            salvo::server::Server::new(acceptor).try_serve(service).await
        } else {
            let acceptor = QuinnListener::new(config.clone(), addr)
                .join(listener)
                .bind()
                .await;
            log::warn!("server started at {addr} with tls");
            salvo::server::Server::new(acceptor).try_serve(service).await
        }
    } else if let Some(ipv6_addr) = ipv6_addr {
        let addr_v6 = SocketAddr::new(IpAddr::V6(ipv6_addr), port);
        let ipv6_listener = TcpListener::new(addr_v6);
        log::warn!("server started at {addr} without tls");
        log::warn!("server started at {addr_v6} without tls");
        // On linux, when the IPv6 addr is unspecified, and IPv4 addr is unspecified, that will cause exception "Address in used"
        #[cfg(not(target_os = "windows"))]
        if ipv6_addr.is_unspecified() {
            let acceptor = ipv6_listener.bind().await;
            salvo::server::Server::new(acceptor).try_serve(service).await
        } else {
            let acceptor = TcpListener::new(addr).join(ipv6_listener).bind().await;
            salvo::server::Server::new(acceptor).try_serve(service).await
        }
        #[cfg(target_os = "windows")]
        {
            let acceptor = TcpListener::new(addr).join(ipv6_listener).bind().await;
            salvo::server::Server::new(acceptor).try_serve(service).await
        }
    } else {
        log::warn!("server started at {addr} without tls");
        let acceptor = TcpListener::new(addr).bind().await;
        salvo::server::Server::new(acceptor).try_serve(service).await
    }
}

#[no_mangle]
pub unsafe extern "C" fn main_get_schema_registry() -> &'static mut SchemaRegistry {
    SchemaRegistry::get_mut()
}
