use anyhow::{anyhow, Error};
use chimes_store_core::config::auth::{AuthorizationConfig, JwtUserClaims};
use chimes_store_core::config::{QueryCondition, QueryObject};
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::redis::{redis_del, redis_delexp_cmd, redis_get, redis_set_expire};
use rbatis::executor::Executor;
use core::future::Future;
use rbatis::Page;
use serde_json::Value;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use super::crud::DbCrud;
use super::crud::DbStoreObject;
use super::query::DbQueryObject;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::service::sdk::{Invocation, RxHookInvoker};

pub struct DbStoreServiceInvocation();

impl DbStoreServiceInvocation {
    fn convert_to_cache_id(&self, uri: &InvokeUri, jwt: &JwtUserClaims, args: &[Value]) -> String {
        let id_body = format!(
            "{}#{}#{}",
            uri.url_no_method(),
            jwt.username,
            serde_json::to_string(args).unwrap_or_default()
        );
        let hash = md5::compute(id_body);
        let hex_hash = format!("{:x}", hash);
        format!("{}-{}-{}", uri.object.clone(), uri.method.clone(), hex_hash)
    }

    fn get_single_to_cache_id(
        &self,
        uri: &InvokeUri,
        method: &str,
        jwt: &JwtUserClaims,
        keyid: &str,
    ) -> String {
        let id_body = format!("{}#{}#{}", uri.url_no_method(), jwt.username, keyid);
        let hash = md5::compute(id_body);
        // let hash = sha2::Sha256::digest(id_body.as_bytes());
        let hex_hash = format!("{:x}", hash);
        format!("{}-{}-{}", uri.object.clone(), method, hex_hash)
    }

    fn get_cache_id_prefix(
        &self,
        uri: &InvokeUri,
        _jwt: &JwtUserClaims,
        _args: &[Value],
    ) -> Vec<String> {
        vec![
            format!("{}-query-", uri.object.clone()),
            format!("{}-paged_query-", uri.object.clone()),
        ]
    }

    fn cache_remove(&self, ns: &str, prefix: &[String], ids: &[String]) {
        for pref in prefix {
            if let Err(err) = redis_delexp_cmd(ns, pref) {
                log::debug!("error for del {} in redis {}", pref, err);
            }
        }

        for pref in ids {
            if let Err(err) = redis_del(ns, pref) {
                log::debug!("error for del {} in redis {}", pref, err);
            }
        }
    }
}

/**
 * Todo: 修复Transaction传递
 * 现在RBatisTxExecutor是每次在执行insert/update/delete操作时，从RBatis中获取，每次执行完成后，该事务都结束
 * 但在实际的应用场景中我们通常需要保持多个CUD操作的事务性一致性，因此，我们使用了InvocationContext，来Hold这个事务
 * 但，由于在Rust中，Pin<Box<dyn Future>> 的异步实现中，是对引用（引用参数）传递有严格的要求，我们无法实现灵活的事务启动与结束机制
 * 即，在声明事务启动，结束后，提交事务
 */
impl Invocation for DbStoreServiceInvocation {
    fn invoke_return_option(
        &'static self,
        uri: &'_ InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &'_ [Value],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let dbs = if let Some(d) = mss.get_object(&uri.object) {
            DbStoreObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            let obj_name = uri.object.clone();
            return Box::pin(async move { Err(anyhow!("Not found by NS {}.", obj_name)) });
        };

        let full_uri = uri.url();

        

        let method = uri.method.clone();
        let local_args = args.to_owned();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();
        let jwt = ctx
            .lock()
            .unwrap()
            .obtain_jwt_user_info()
            .unwrap_or(JwtUserClaims::anonymous());

        let ns = uri.namespace.clone();

        let tx_opt = ctx.lock().unwrap().get_tx_executor_sync(&ns);

        if method == *"insert"
            || method == *"update"
            || method == *"delete"
            || method == *"upsert"
            || method == *"save_batch"
            || method == *"update_by"
            || method == *"delete_by"
        {
            if dbs.0.enable_cache {
                let cache_ids = self.get_cache_id_prefix(uri, &jwt, args);
                for id in cache_ids {
                    if let Err(err) = redis_delexp_cmd(&uri.namespace, &id) {
                        log::info!("error on redis delete {:?}", err);
                    }
                }
            }
            let rb_ = mss.get_rbatis();
            
            match method_str {
                "insert" => {
                    let prefixes = self.get_cache_id_prefix(uri, &jwt, args);
                    Box::pin(async move {
                        if let Ok(conn) = rb_.acquire().await {
                            ctx.lock().unwrap().set_rbatis_connection(&ns, Arc::new(conn));
                        }

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.insert_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;

                        // ctx.lock().unwrap().insert("key", Arc::clone(&tx_));
                        match dbs.insert(tx, &jwt, &pass_args[0]).await {
                            Ok(v) => {
                                ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.insert_hooks.clone(),
                                    ctx.clone(),
                                    pass_args.clone(),
                                )
                                .await?;
                                if dbs.0.enable_cache {
                                    self.cache_remove(&ns, &prefixes, &[]);
                                }
                                match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                    Ok(ts) => Ok(ts.to_owned()),
                                    Err(_) => Ok(Some(v)),
                                }
                            }
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.insert_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                }
                "update" => {
                    // the args[0] will be hooked update to another value, that may be error
                    let prefixes = if dbs.0.enable_cache {
                        self.get_cache_id_prefix(uri, &jwt, args)
                    } else {
                        vec![]
                    };

                    let cache_id = if dbs.0.enable_cache {
                        self.get_single_to_cache_id(
                            uri,
                            "select",
                            &jwt,
                            &dbs.get_pkey_value_present(&args[0]).unwrap_or_default(),
                        )
                    } else {
                        "-----------------------------------------".to_owned()
                    };

                    Box::pin(async move {
                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.update_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;

                        match dbs.update(tx, &jwt, &pass_args[0]).await {
                            Ok(v) => {
                                ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.update_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                if dbs.0.enable_cache {
                                    self.cache_remove(&ns, &prefixes, &[cache_id]);
                                }
                                Ok(Some(v))
                            }
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.update_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                }
                "upsert" => {
                    let prefixes = if dbs.0.enable_cache {
                        self.get_cache_id_prefix(uri, &jwt, args)
                    } else {
                        vec![]
                    };

                    let cache_id = if dbs.0.enable_cache {
                        self.get_single_to_cache_id(
                            uri,
                            "select",
                            &jwt,
                            &dbs.get_pkey_value_present(&args[0]).unwrap_or_default(),
                        )
                    } else {
                        "-------------------------------------------".to_owned()
                    };
                    Box::pin(async move {
                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.upsert_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let qs = if pass_args.len() == 1 {
                            None
                        } else {
                            match dbs.to_condition(&pass_args[1]) {
                                Ok(t) => Some(t),
                                Err(_) => None,
                            }
                        };

                        match dbs.upsert(tx, &jwt, &pass_args[0], qs).await {
                            Ok(v) => {
                                ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.upsert_hooks.clone(),
                                    ctx.clone(),
                                    pass_args.clone(),
                                )
                                .await?;

                                if dbs.0.enable_cache {
                                    self.cache_remove(&ns, &prefixes, &[cache_id]);
                                }
                                match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                    Ok(ts) => Ok(ts.to_owned()),
                                    Err(_) => Ok(Some(v)),
                                }
                            }
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.upsert_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                },
                "save_batch" => {
                    let prefixes = if dbs.0.enable_cache {
                        self.get_cache_id_prefix(uri, &jwt, args)
                    } else {
                        vec![]
                    };

                    let cache_id = if dbs.0.enable_cache {
                        self.get_single_to_cache_id(
                            uri,
                            "select",
                            &jwt,
                            &dbs.get_pkey_value_present(&args[0]).unwrap_or_default(),
                        )
                    } else {
                        "-------------------------------------------".to_owned()
                    };
                    Box::pin(async move {
                        let _ = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.savebatch_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;

                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.upsert_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };
                        let mut ctret_vals = vec![];
                        for ctval in pass_args.clone() {
                            let qst = if let Some(qsval) = ctval.get("_cond") {
                                if let Ok(qs) = serde_json::from_value::<QueryCondition>(qsval.to_owned()) {
                                    Some(qs)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            let ctret = match dbs.upsert(tx.clone(), &jwt, &ctval, qst).await {
                                Ok(v) => {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri.clone(),
                                        dbs.0.upsert_hooks.clone(),
                                        ctx.clone(),
                                        pass_args.clone(),
                                    )
                                    .await?;

                                    if dbs.0.enable_cache {
                                        self.cache_remove(&ns, &prefixes, &[cache_id.clone()]);
                                    }
                                    match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                        Ok(ts) => Ok(ts.to_owned()),
                                        Err(_) => Ok(Some(v)),
                                    }
                                }
                                Err(err) => {
                                    ctx.lock().unwrap().set_failed();
                                    ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri.clone(),
                                        dbs.0.upsert_hooks.clone(),
                                        ctx.clone(),
                                        pass_args.clone(),
                                    )
                                    .await?;
                                    Err(err)
                                }
                            };
                            
                            if ctret.is_err() {
                                return ctret;
                            } else if let Ok(Some(tt)) = ctret {
                                ctret_vals.push(tt);
                            }
                        }


                        
                        ctx.lock().unwrap().insert("RETURN_VALUE", Some(ctret_vals.clone()));
                        let _ = MxStoreService::invoke_post_hook_(
                            full_uri.clone(),
                            dbs.0.upsert_hooks.clone(),
                            ctx.clone(),
                            pass_args.clone(),
                        )
                        .await?;

                        let okret = Some(Value::Array(ctret_vals));
                        Ok(okret)
                    })
                },
                "update_by" => {
                    let uri_cp = uri.clone();
                    let prefixes = if dbs.0.enable_cache {
                        self.get_cache_id_prefix(uri, &jwt, args)
                    } else {
                        vec![]
                    };

                    Box::pin(async move {
                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.update_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;

                        let qs = if pass_args.len() == 1 {
                            ctx.lock().unwrap().insert(
                                "EXCEPTION",
                                "No QueryCondition provided for update_by".to_string(),
                            );
                            let _ = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.update_hooks.clone(),
                                ctx,
                                pass_args.clone(),
                            )
                            .await?;
                            return Err(anyhow!("No QueryCondition provided for update_by"));
                        } else {
                            match dbs.to_condition(&pass_args[1]) {
                                Ok(t) => t,
                                Err(err) => {
                                    ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri,
                                        dbs.0.update_hooks.clone(),
                                        ctx,
                                        pass_args.clone(),
                                    )
                                    .await?;
                                    return Err(anyhow!(err));
                                }
                            }
                        };

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let update_vec = match dbs.query(Arc::new(rb_.clone()), &jwt, &qs).await {
                            Ok(ts) => ts,
                            Err(err) => {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.update_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                return Err(anyhow!(err));
                            }
                        };

                        match dbs.update_by(tx, &jwt, &pass_args[0], &qs).await {
                            Ok(v) => {
                                ctx.clone()
                                    .lock()
                                    .unwrap()
                                    .insert("rows_affected", Some(v.clone()));
                                let mut cached_ids = vec![];
                                for tc in update_vec {
                                    ctx.clone()
                                        .lock()
                                        .unwrap()
                                        .insert("RETURN_VALUE", Some(tc.clone()));
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri.clone(),
                                        dbs.0.update_hooks.clone(),
                                        ctx.clone(),
                                        pass_args.clone(),
                                    )
                                    .await?;
                                    if dbs.0.enable_cache {
                                        let cid = self.get_single_to_cache_id(
                                            &uri_cp,
                                            "select",
                                            &jwt,
                                            &dbs.get_pkey_value_present(&tc).unwrap_or_default(),
                                        );
                                        cached_ids.push(cid);
                                    }
                                }

                                if dbs.0.enable_cache {
                                    self.cache_remove(&ns, &prefixes, &cached_ids);
                                }
                                Ok(Some(v))
                            }
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.update_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                }
                "delete" => {
                    let uri_cp = uri.clone();
                    let prefixes = if dbs.0.enable_cache {
                        self.get_cache_id_prefix(uri, &jwt, args)
                    } else {
                        vec![]
                    };

                    let cid = if dbs.0.enable_cache {
                        self.get_single_to_cache_id(
                            &uri_cp,
                            "select",
                            &jwt,
                            &dbs.get_pkey_value_present(&args[0]).unwrap_or_default(),
                        )
                    } else {
                        "------------------------------------".to_owned()
                    };

                    Box::pin(async move {
                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.delete_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;
                    
                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                    
                        match dbs.delete(tx, &jwt, &pass_args[0]).await {
                            Ok(v) => {
                                ctx.clone()
                                    .lock()
                                    .unwrap()
                                    .insert("rows_affected", Some(v.clone()));
                                ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.delete_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                if dbs.0.enable_cache {
                                    self.cache_remove(&ns, &prefixes, &[cid]);
                                }
                                Ok(Some(v))
                            }
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.delete_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                }
                "delete_by" => {
                    let uri_cp = uri.clone();
                    let prefixes = if dbs.0.enable_cache {
                        self.get_cache_id_prefix(uri, &jwt, args)
                    } else {
                        vec![]
                    };

                    Box::pin(async move {
                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.delete_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;
                                                
                        let qs = if pass_args.len() < 2 {
                            ctx.lock().unwrap().insert(
                                "EXCEPTION",
                                "No QueryCondition provided for update_by".to_string(),
                            );
                            let _ = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.delete_hooks.clone(),
                                ctx,
                                pass_args.clone(),
                            )
                            .await?;
                            return Err(anyhow!("No QueryCondition provided for update_by"));
                        } else {
                            match dbs.to_condition(&pass_args[1]) {
                                Ok(t) => t,
                                Err(err) => {
                                    ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri,
                                        dbs.0.delete_hooks.clone(),
                                        ctx,
                                        pass_args.clone(),
                                    )
                                    .await?;
                                    return Err(anyhow!(err));
                                }
                            }
                        };
                  
                        let update_vec = match dbs.query(Arc::new(rb_.clone()), &jwt, &qs).await {
                            Ok(ts) => ts,
                            Err(err) => {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.delete_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                return Err(anyhow!(err));
                            }
                        };

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let con = rb_.acquire_begin().await?;
                            let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        match dbs.delete_by(tx, &jwt, &qs).await {
                            Ok(v) => {
                                // v.get("rows_affected")
                                ctx.clone()
                                    .lock()
                                    .unwrap()
                                    .insert("rows_affected", Some(v.clone()));
                                let mut cached_ids = vec![];
                                for tc in update_vec {
                                    ctx.clone()
                                        .lock()
                                        .unwrap()
                                        .insert("RETURN_VALUE", Some(tc.clone()));
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri.clone(),
                                        dbs.0.delete_hooks.clone(),
                                        ctx.clone(),
                                        pass_args.clone(),
                                    )
                                    .await?;
                                    if dbs.0.enable_cache {
                                        let cid = self.get_single_to_cache_id(
                                            &uri_cp,
                                            "select",
                                            &jwt,
                                            &dbs.get_pkey_value_present(&tc).unwrap_or_default(),
                                        );
                                        cached_ids.push(cid);
                                    }
                                }

                                if dbs.0.enable_cache {
                                    self.cache_remove(&ns, &prefixes, &cached_ids);
                                }
                                Ok(Some(v))
                            }
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.delete_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                }
                _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
            }
        } else {
            let key_id = dbs.get_pkey_value_present(&args[0]);
            let cache_id = self.get_single_to_cache_id(
                uri,
                &uri.method.clone(),
                &jwt,
                &key_id.unwrap_or_default(),
            );

            let rb_ = mss.get_rbatis();
            let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&ns);
            
            match method_str {
                "select" => Box::pin(async move {
                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Value>(&cache_ret) {
                                if !ret.is_null() {
                                    return Ok(Some(ret));
                                }
                            }
                        }
                    }

                    let conn = if let Some(conn) = conn_opt {
                        conn
                    } else {
                        let con = rb_.acquire().await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let pass_args = MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.select_hooks.clone(),
                        ctx.clone(),
                        local_args.to_vec(),
                    )
                    .await?;
                    // dbs.select(&rb_, &pass_args[0]).await
                    let fix_arg = if !pass_args.is_empty() {
                        pass_args[0].to_owned()
                    } else {
                        local_args[0].to_owned()
                    };
                    
                    match dbs.select(conn, &jwt, &fix_arg).await {
                        Ok(ts) => {
                            ctx.lock().unwrap().insert("RETURN_VALUE", ts.clone());
                            MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.select_hooks.clone(),
                                ctx.clone(),
                                pass_args,
                            )
                            .await?;
                            match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                Ok(ts) => {
                                    if dbs.0.enable_cache && ts.is_some() {
                                        if let Ok(text) =
                                            serde_json::to_string(&ts.clone().unwrap())
                                        {
                                            if let Err(err) = redis_set_expire(
                                                &ns,
                                                &cache_id,
                                                &text,
                                                dbs.0.cache_time.unwrap_or(30) as u64,
                                            ) {
                                                log::info!("error for cache {}", err);
                                            }
                                        }
                                    }
                                    Ok(ts.to_owned())
                                }
                                Err(_) => Ok(ts),
                            }
                        }
                        Err(err) => {
                            ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                            let _ = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.select_hooks.clone(),
                                ctx,
                                pass_args.clone(),
                            )
                            .await?;
                            Err(err)
                        }
                    }
                }),
                "find_one" => {
                    Box::pin(async move {
                        if dbs.0.enable_cache {
                            if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                                if let Ok(ret) = serde_json::from_str::<Value>(&cache_ret) {
                                    if !ret.is_null() {
                                        return Ok(Some(ret));
                                    }
                                }
                            }
                        }
                        
                        let conn = if let Some(conn) = conn_opt {
                            conn
                        } else {
                            let con = rb_.acquire().await?;
                            let xcon: Arc<dyn Executor> = Arc::new(con);
                            ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                            xcon
                        };

                        let pass_args = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.select_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await?;
                        // dbs.select(&rb_, &pass_args[0]).await
                        let fix_arg = if !pass_args.is_empty() {
                            pass_args[0].to_owned()
                        } else {
                            local_args[0].to_owned()
                        };
                        
                        match dbs.to_condition(&fix_arg) {
                            Ok(qs) => match dbs.find_one(conn, &jwt, &qs).await {
                                Ok(ts) => {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", ts.clone());
                                    MxStoreService::invoke_post_hook_(
                                        full_uri,
                                        dbs.0.select_hooks.clone(),
                                        ctx.clone(),
                                        pass_args,
                                    )
                                    .await?;
                                    match ctx.lock().unwrap().get::<Option<Value>>("RETURN_VALUE") {
                                        Ok(ts) => {
                                            if dbs.0.enable_cache && ts.is_some() {
                                                if let Ok(text) =
                                                    serde_json::to_string(&ts.clone().unwrap())
                                                {
                                                    if let Err(err) = redis_set_expire(
                                                        &ns,
                                                        &cache_id,
                                                        &text,
                                                        dbs.0.cache_time.unwrap_or(30) as u64,
                                                    ) {
                                                        log::info!("error for cache {}", err);
                                                    }
                                                }
                                            }
                                            Ok(ts.to_owned())
                                        }
                                        Err(_) => Ok(ts),
                                    }
                                }
                                Err(err) => {
                                    ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                    let _ = MxStoreService::invoke_post_hook_(
                                        full_uri,
                                        dbs.0.select_hooks.clone(),
                                        ctx,
                                        pass_args.clone(),
                                    )
                                    .await?;
                                    Err(err)
                                }
                            },
                            Err(err) => {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.select_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        }
                    })
                }
                _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
            }
        }
    }

    fn invoke_return_vec(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let dbs = if let Some(d) = mss.get_object(&uri.object) {
            DbStoreObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            let obj_name = uri.object.clone();
            return Box::pin(async move { Err(anyhow!("Not found by NS uri.object {obj_name}.")) });
        };

        let method = uri.method.clone();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();

        let rb_ = mss.get_rbatis();

        let full_uri = uri.url();
        let ns = uri.namespace.clone();

        let local_args = args.to_owned();

        let jwt = ctx
            .lock()
            .unwrap()
            .obtain_jwt_user_info()
            .unwrap_or(JwtUserClaims::anonymous());

        let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&ns);

        match method_str {
            "query" => {
                let cache_id = self.convert_to_cache_id(uri, &jwt, args);
                Box::pin(async move {
                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Value>(&cache_ret) {
                                if !ret.is_null() && ret.is_array() {
                                    if let Some(tx) = ret.as_array() {
                                        return Ok(tx.to_owned());
                                    }
                                }
                            }
                        }
                    }
                    
                    let conn = if let Some(conn) = conn_opt {
                        conn
                    } else {
                        let con = rb_.acquire().await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let pass_args = MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.query_hooks.clone(),
                        ctx.clone(),
                        local_args.clone(),
                    )
                    .await?;
                    // dbs.select(&rb_, &pass_args[0]).await
                    let fix_arg = if !pass_args.is_empty() {
                        pass_args[0].to_owned()
                    } else {
                        local_args[0].to_owned()
                    };
                    match dbs.to_condition(&fix_arg) {
                        Ok(qs) => match dbs.query(conn, &jwt, &qs).await {
                            Ok(ts) => {
                                ctx.lock().unwrap().insert("RETURN_VALUE", ts.clone());
                                MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.query_hooks.clone(),
                                    ctx.clone(),
                                    pass_args,
                                )
                                .await?;
                                match ctx.lock().unwrap().get::<Vec<Value>>("RETURN_VALUE") {
                                    Ok(ts) => {
                                        if dbs.0.enable_cache {
                                            let tc = ts.clone();
                                            if let Ok(text) = serde_json::to_string(&tc) {
                                                if let Err(err) = redis_set_expire(
                                                    &ns,
                                                    &cache_id,
                                                    &text,
                                                    dbs.0.cache_time.unwrap_or(30) as u64,
                                                ) {
                                                    log::info!("Error for redis set {}", err);
                                                }
                                            }
                                        }
                                        Ok(ts.to_owned())
                                    }
                                    Err(_) => Ok(ts),
                                }
                            }
                            Err(err) => {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.delete_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        },
                        Err(err) => {
                            ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                            let _ = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.delete_hooks.clone(),
                                ctx,
                                pass_args.clone(),
                            )
                            .await?;
                            Err(err)
                        }
                    }
                })
            }
            _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
        }
    }

    fn invoke_return_page(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let dbs = if let Some(d) = mss.get_object(&uri.object) {
            DbStoreObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            let obj_name = uri.object.clone();
            return Box::pin(async move { Err(anyhow!("Not found by NS uri.object {obj_name}.")) });
        };

        let method = uri.method.clone();
        // let local_args = args.to_owned();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();

        let rb_ = mss.get_rbatis();

        let full_uri = uri.url();
        let ns = uri.namespace.clone();

        let local_args = args.to_owned();
        let jwt = ctx
            .lock()
            .unwrap()
            .obtain_jwt_user_info()
            .unwrap_or(JwtUserClaims::anonymous());
        
        let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&ns);

        match method_str {
            "paged_query" => {
                let cache_id = self.convert_to_cache_id(uri, &jwt, args);
                Box::pin(async move {
                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Page<Value>>(&cache_ret) {
                                return Ok(ret);
                            }
                        }
                    }

                    let conn = if let Some(conn) = conn_opt {
                        conn
                    } else {
                        let con = rb_.acquire().await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let pass_args = MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.query_hooks.clone(),
                        ctx.clone(),
                        local_args.clone(),
                    )
                    .await?;
                    // dbs.select(&rb_, &pass_args[0]).await
                    let fix_arg = if !pass_args.is_empty() {
                        pass_args[0].to_owned()
                    } else {
                        local_args[0].to_owned()
                    };
                    match dbs.to_condition(&fix_arg) {
                        Ok(qs) => match dbs.paged_query(conn, &jwt, &qs).await {
                            Ok(ts) => {
                                ctx.lock().unwrap().insert("RETURN_VALUE", ts.clone());
                                MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.query_hooks.clone(),
                                    ctx.clone(),
                                    pass_args,
                                )
                                .await?;
                                match ctx.lock().unwrap().get::<Page<Value>>("RETURN_VALUE") {
                                    Ok(ts) => {
                                        if dbs.0.enable_cache {
                                            let tc = ts.clone();
                                            if let Ok(text) = serde_json::to_string(&tc) {
                                                if let Err(err) = redis_set_expire(
                                                    &ns,
                                                    &cache_id,
                                                    &text,
                                                    dbs.0.cache_time.unwrap_or(30) as u64,
                                                ) {
                                                    log::info!("Error for redis set {}", err);
                                                }
                                            }
                                        }
                                        Ok(ts.to_owned())
                                    }
                                    Err(_) => Ok(ts),
                                }
                            }
                            Err(err) => {
                                ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                                let _ = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.delete_hooks.clone(),
                                    ctx,
                                    pass_args.clone(),
                                )
                                .await?;
                                Err(err)
                            }
                        },
                        Err(err) => {
                            ctx.lock().unwrap().insert("EXCEPTION", err.to_string());
                            let _ = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.delete_hooks.clone(),
                                ctx,
                                pass_args.clone(),
                            )
                            .await?;
                            Err(err)
                        }
                    }
                })
            }
            _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
        }
    }
}

pub struct DbQueryServiceInvocation();

impl DbQueryServiceInvocation {
    fn convert_to_cache_id(&self, uri: &InvokeUri, jwt: &JwtUserClaims, args: &[Value]) -> String {
        let id_body = format!(
            "{}#{}#{}",
            uri.url_no_method(),
            jwt.username,
            serde_json::to_string(args).unwrap_or_default()
        );
        let hash = md5::compute(id_body);
        let hex_hash = format!("{:x}", hash);
        format!("{}-{}-{}", uri.object.clone(), uri.method.clone(), hex_hash)
    }
}

impl Invocation for DbQueryServiceInvocation {
    fn invoke_return_option(
        &'static self,
        _uri: &'_ InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: &'_ [Value],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, Error>> + Send>> {
        Box::pin(async { Err(anyhow!("Not implemented")) })
    }

    fn invoke_return_vec(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let dbs = if let Some(d) = mss.get_query(&uri.object) {
            DbQueryObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        // mss.update_invocation_ctx(ctx);
        // mss.update_invocation_ctx_locked(ctx.clone());
        let method = uri.method.clone();
        let ns = uri.namespace.clone();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();

        // let rb_ = if let Ok(r) = ctx.lock().unwrap().get_rbatis() {
        //     r.to_owned()
        // } else {
        //     mss.get_rbatis().to_owned()
        // };

        let rb_ = mss.get_rbatis(); // .to_owned();

        let jwt = ctx
            .lock()
            .unwrap()
            .obtain_jwt_user_info()
            .unwrap_or(JwtUserClaims::anonymous());

        let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&ns);

        match method_str {
            "search" => {
                let full_url = uri.url();
                let pass_args = args.to_vec();
                let cache_id = self.convert_to_cache_id(uri, &jwt, args);
                log::info!("search {}", cache_id);
                Box::pin(async move {
                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Vec<Value>>(&cache_ret) {
                                return Ok(ret);
                            }
                        }
                    }

                    let conn = if let Some(conn) = conn_opt {
                        conn
                    } else {
                        let con = rb_.acquire().await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let mix_args = MxStoreService::invoke_pre_hook_(
                        full_url.clone(),
                        dbs.0.hooks.clone(),
                        ctx.clone(),
                        pass_args.clone(),
                    )
                    .await?;
                    let fix_arg = if !mix_args.is_empty() {
                        mix_args[0].to_owned()
                    } else {
                        pass_args[0].to_owned()
                    };
                    let rb_ = mss.get_rbatis().to_owned();
                    if let Ok(qs) = dbs.to_condition(&pass_args) {
                        match dbs.query(conn, &jwt, &fix_arg, &qs).await {
                            Ok(rs) => {
                                drop(rb_);
                                if !dbs.0.hooks.is_empty() {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", rs.clone());
                                    MxStoreService::invoke_post_hook_(
                                        full_url,
                                        dbs.0.hooks.clone(),
                                        ctx.clone(),
                                        pass_args,
                                    )
                                    .await?;
                                    match ctx.lock().unwrap().get::<Vec<Value>>("RETURN_VALUE") {
                                        Ok(retval) => {
                                            if dbs.0.enable_cache {
                                                let tc = retval.clone();
                                                if let Ok(text) = serde_json::to_string(&tc) {
                                                    // log::info!("set redis {} = {}", cache_id, text);
                                                    if let Err(err) = redis_set_expire(
                                                        &ns,
                                                        &cache_id,
                                                        &text,
                                                        dbs.0.cache_time.unwrap_or(30) as u64,
                                                    ) {
                                                        log::info!("Error for redis set {}", err);
                                                    }
                                                }
                                            }
                                            Ok(retval.to_owned())
                                        }
                                        Err(_) => {
                                            Err(anyhow!("Could not GET/CAST the RETURN_VALUE"))
                                        }
                                    }
                                } else {
                                    if dbs.0.enable_cache {
                                        let tc = rs.clone();
                                        if let Ok(text) = serde_json::to_string(&tc) {
                                            // log::info!("set redis {} = {}", cache_id, text);
                                            if let Err(err) = redis_set_expire(
                                                &ns,
                                                &cache_id,
                                                &text,
                                                dbs.0.cache_time.unwrap_or(30) as u64,
                                            ) {
                                                log::info!("Error for redis set {}", err);
                                            }
                                        }
                                    }
                                    Ok(rs)
                                }
                            }
                            Err(err) => {
                                drop(rb_);
                                Err(err)
                            },
                        }
                    } else {
                        Ok(vec![])
                    }
                })
            }
            _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
        }
    }

    fn invoke_return_page(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &[Value],
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let dbs = if let Some(d) = mss.get_query(&uri.object) {
            DbQueryObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let method = uri.method.clone();
        let ns = uri.namespace.clone();
        // let local_args = args.to_owned();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();

        let rb_ = mss.get_rbatis().to_owned();

        let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&ns);

        let jwt = ctx
            .lock()
            .unwrap()
            .obtain_jwt_user_info()
            .unwrap_or(JwtUserClaims::anonymous());

        match method_str {
            "paged_search" => {
                let pass_args = args.to_vec();
                let full_uri = uri.url();
                let cache_id = self.convert_to_cache_id(uri, &jwt, args);
                Box::pin(async move {
                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Page<Value>>(&cache_ret) {
                                return Ok(ret);
                            }
                        }
                    }

                    let conn = if let Some(conn) = conn_opt {
                        conn
                    } else {
                        let con = rb_.acquire().await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let mix_args = MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.hooks.clone(),
                        ctx.clone(),
                        pass_args.clone(),
                    )
                    .await?;
                    let fix_arg = mix_args[0].to_owned();
                    if let Ok(qs) = dbs.to_condition(&mix_args) {
                        let ret = dbs.paged_query(conn, &jwt, &fix_arg, &qs).await?;
                        if !dbs.0.hooks.is_empty() {
                            ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                            MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.hooks.clone(),
                                ctx.clone(),
                                pass_args,
                            )
                            .await?;
                            match ctx.lock().unwrap().get::<Page<Value>>("RETURN_VALUE") {
                                Ok(retval) => {
                                    if dbs.0.enable_cache {
                                        let tc = retval.clone();
                                        if let Ok(text) = serde_json::to_string(&tc) {
                                            if let Err(err) = redis_set_expire(
                                                &ns,
                                                &cache_id,
                                                &text,
                                                dbs.0.cache_time.unwrap_or(30) as u64,
                                            ) {
                                                log::info!("Error for redis set {}", err);
                                            }
                                        }
                                    }
                                    Ok(retval.to_owned())
                                }
                                Err(_) => Err(anyhow!("Could not GET/CAST the RETURN_VALUE")),
                            }
                        } else {
                            if dbs.0.enable_cache {
                                let tc = ret.clone();
                                if let Ok(text) = serde_json::to_string(&tc) {
                                    // log::info!("set redis {} = {}", cache_id, text);
                                    if let Err(err) = redis_set_expire(
                                        &ns,
                                        &cache_id,
                                        &text,
                                        dbs.0.cache_time.unwrap_or(30) as u64,
                                    ) {
                                        log::info!("Error for redis set {}", err);
                                    }
                                }
                            }
                            Ok(ret)
                        }
                    } else {
                        Ok(Page::new(0, 10))
                    }
                })
            }
            _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
        }
    }

    fn invoke_direct_query(
        &'static self,
        namespace: String,
        ctx: Arc<Mutex<InvocationContext>>,
        query: String,
        args: Vec<Value>) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
            let mss = if let Some(t) = MxStoreService::get(&namespace) {
                t
            } else {
                return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
            };
    
            let dbs = DbQueryObject(QueryObject::default(), mss.get_config(), AuthorizationConfig::get());
    
            let rb_ = mss.get_rbatis(); // .to_owned();
            let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&namespace);
            let ns_ = namespace.clone();
            Box::pin(async move {
                let conn = if let Some(conn) = conn_opt {
                    conn
                } else {
                    let con = rb_.acquire().await?;
                    let xcon: Arc<dyn Executor> = Arc::new(con);
                    ctx.lock().unwrap().set_rbatis_connection(&ns_, xcon.clone());
                    xcon
                };

                dbs.direct_query(conn, &query, &args).await
            })
    }
}
