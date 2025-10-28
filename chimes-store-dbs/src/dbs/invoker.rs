use anyhow::{anyhow, Error};
use chimes_store_core::config::auth::{AuthorizationConfig, JwtUserClaims};
use chimes_store_core::config::{QueryCondition, QueryObject};
use chimes_store_core::service::starter::MxStoreService;
use chimes_store_core::utils::algorithm::unwrap_json_string;
use chimes_store_core::utils::get_multiple_rbatis_async;
use chimes_store_core::utils::redis::{redis_del, redis_delexp_cmd, redis_get, redis_set_expire};
use core::future::Future;
use rbatis::executor::Executor;
use rbatis::Page;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::crud::DbCrud;
use super::crud::DbStoreObject;
use super::query::DbQueryObject;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::InvokeUri;
use chimes_store_core::service::sdk::{Invocation, RxHookInvoker};

pub const ACQUIRE_TIMEOUT: u64 = 30;

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
        let hex_hash = format!("{hash:x}");
        format!("{}-{}-{}", uri.object.clone(), uri.method.clone(), hex_hash)
    }

    fn get_single_to_cache_id(
        &self,
        uri: &InvokeUri,
        method: &str,
        _jwt: &JwtUserClaims,
        keyid: &str,
    ) -> String {
        let id_body = format!("{}#{}#{}", uri.url_no_method(), "allusers", keyid);
        let hash = md5::compute(id_body);
        // let hash = sha2::Sha256::digest(id_body.as_bytes());
        let hex_hash = format!("{hash:x}");
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
                log::debug!("error for del {pref} in redis {err}");
            }
        }

        for pref in ids {
            if let Err(err) = redis_del(ns, pref) {
                log::debug!("error for del {pref} in redis {err}");
            }
        }
    }
}

/**
 * 现在RBatisTxExecutor是每次在执行insert/update/delete操作时，从RBatis中获取，每次执行完成后，该事务都结束
 * 但在实际的应用场景中我们通常需要保持多个CUD操作的事务性一致性，因此，我们使用了InvocationContext，来Hold这个事务
 * 但，由于在Rust中，Pin<Box<dyn Future>> 的异步实现中，是对引用（引用参数）传递有严格的要求，我们无法实现灵活的事务启动与结束机制
 * 即，在声明事务启动，结束后，提交事务。
 * 所以，目前来说，InvocationContent中可以保留了执行序列中的事务处理能力，
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
                        log::info!("error on redis delete {err:?}");
                    }
                }
            }
            // let rb_ = mss.get_rbatis();
            let def_db_url = mss.get_db_url();

            match method_str {
                "insert" => {
                    let prefixes = self.get_cache_id_prefix(uri, &jwt, args);
                    Box::pin(async move {
                        // TODO: should verify this behavior is nessary.
                        // if let Ok(conn) = rb_.try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT)).await {
                        //     ctx.lock()
                        //         .unwrap()
                        //         .set_rbatis_connection(&ns, Arc::new(conn));
                        // }

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            // let con = rb_.acquire_begin().await?;
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);
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
                                if let Err(err) = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.insert_hooks.clone(),
                                    ctx.clone(),
                                    pass_args.clone(),
                                )
                                .await
                                {
                                    ctx.lock().unwrap().set_failed();
                                    return Err(err);
                                }

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
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);

                            // let con = rb_.acquire_begin().await?;
                            // let txc = Arc::new(con);
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
                                if let Err(err) = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.update_hooks.clone(),
                                    ctx.clone(),
                                    pass_args.clone(),
                                )
                                .await
                                {
                                    ctx.lock().unwrap().set_failed();
                                    return Err(err);
                                }

                                log::error!("When success called update. the redis of {cache_id} will be removed.");

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
                        let pass_args = match MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.upsert_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await
                        {
                            Ok(t) => t,
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                return Err(err);
                            }
                        };

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);

                            // let con = rb_.acquire_begin().await?;
                            // let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let qs = if pass_args.len() == 1 {
                            let qc_cond = pass_args[0].clone();
                            if let Some(qccond) = qc_cond.get("_cond") {
                                dbs.to_condition(qccond).ok()
                            } else {
                                None
                            }
                        } else if pass_args.len() > 1 {
                            dbs.to_condition(&pass_args[1]).ok()
                        } else {
                            Some(QueryCondition::default())
                        };

                        match dbs.upsert(tx, &jwt, &pass_args[0], qs).await {
                            Ok(v) => {
                                ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                if let Err(err) = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.upsert_hooks.clone(),
                                    ctx.clone(),
                                    pass_args.clone(),
                                )
                                .await
                                {
                                    ctx.lock().unwrap().set_failed();
                                    return Err(err);
                                }

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
                }
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
                        if let Err(err) = MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.savebatch_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await
                        {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }

                        let pass_args = match MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.upsert_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await
                        {
                            Ok(vt) => vt,
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                return Err(err);
                            }
                        };

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);

                            // let con = rb_.acquire_begin().await?;
                            // let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };
                        let mut ctret_vals = vec![];
                        for ctval in pass_args.clone() {
                            let qst = if let Some(qsval) = ctval.get("_cond") {
                                serde_json::from_value::<QueryCondition>(qsval.to_owned()).ok()
                            } else {
                                None
                            };
                            let ctret = match dbs.upsert(tx.clone(), &jwt, &ctval, qst).await {
                                Ok(v) => {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", Some(v.clone()));
                                    if let Err(err) = MxStoreService::invoke_post_hook_(
                                        full_uri.clone(),
                                        dbs.0.upsert_hooks.clone(),
                                        ctx.clone(),
                                        pass_args.clone(),
                                    )
                                    .await
                                    {
                                        ctx.lock().unwrap().set_failed();
                                        return Err(err);
                                    }

                                    if dbs.0.enable_cache {
                                        self.cache_remove(
                                            &ns,
                                            &prefixes,
                                            std::slice::from_ref(&cache_id),
                                        );
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

                        ctx.lock()
                            .unwrap()
                            .insert("RETURN_VALUE", Some(ctret_vals.clone()));
                        if let Err(err) = MxStoreService::invoke_post_hook_(
                            full_uri.clone(),
                            dbs.0.upsert_hooks.clone(),
                            ctx.clone(),
                            pass_args.clone(),
                        )
                        .await
                        {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }

                        let okret = Some(Value::Array(ctret_vals));
                        Ok(okret)
                    })
                }
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
                            ctx.lock().unwrap().set_failed();
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
                        } else if pass_args.len() > 1 {
                            match dbs.to_condition(&pass_args[1]) {
                                Ok(t) => t,
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
                                    return Err(anyhow!(err));
                                }
                            }
                        } else {
                            QueryCondition::default()
                        };

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);
                            // let con = rb_.acquire_begin().await?;
                            // let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let update_vec = match dbs.query(tx.clone(), &jwt, &qs).await {
                            Ok(ts) => ts,
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

                                    if let Err(err) = MxStoreService::invoke_post_hook_(
                                        full_uri.clone(),
                                        dbs.0.update_hooks.clone(),
                                        ctx.clone(),
                                        pass_args.clone(),
                                    )
                                    .await
                                    {
                                        ctx.lock().unwrap().set_failed();
                                        return Err(err);
                                    }

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
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);

                            // let con = rb_.acquire_begin().await?;
                            // let txc = Arc::new(con);
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
                            ctx.lock().unwrap().set_failed();
                            ctx.lock().unwrap().insert(
                                "EXCEPTION",
                                "No QueryCondition provided for delete_by".to_string(),
                            );
                            let _ = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.delete_hooks.clone(),
                                ctx,
                                pass_args.clone(),
                            )
                            .await?;
                            return Err(anyhow!("No QueryCondition provided for delete_by"));
                        } else {
                            match dbs.to_condition(&pass_args[1]) {
                                Ok(t) => t,
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
                                    return Err(anyhow!(err));
                                }
                            }
                        };

                        let tx = if let Some(conn) = tx_opt {
                            conn
                        } else {
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let tcon = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let con_tx = tcon.begin().await?;
                            let txc = Arc::new(con_tx);

                            // let con = rb_.acquire_begin().await?;
                            // let txc = Arc::new(con);
                            ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                            txc
                        };

                        let update_vec = match dbs.query(tx.clone(), &jwt, &qs).await {
                            Ok(ts) => ts,
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
                                return Err(anyhow!(err));
                            }
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
                                    .await
                                    .unwrap_or_default(); // avoid error is best way?
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

            // let rb_ = mss.get_rbatis();
            let def_db_url = mss.get_db_url();
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
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let pass_args = match MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.select_hooks.clone(),
                        ctx.clone(),
                        local_args.to_vec(),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }
                    };

                    // dbs.select(&rb_, &pass_args[0]).await
                    let fix_arg = if !pass_args.is_empty() {
                        pass_args[0].to_owned()
                    } else {
                        local_args[0].to_owned()
                    };

                    match dbs.select(conn, &jwt, &fix_arg).await {
                        Ok(ts) => {
                            ctx.lock().unwrap().insert("RETURN_VALUE", ts.clone());
                            if let Err(err) = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.select_hooks.clone(),
                                ctx.clone(),
                                pass_args,
                            )
                            .await
                            {
                                ctx.lock().unwrap().set_failed();
                                return Err(err);
                            }

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
                                                log::info!("error for cache {err}");
                                            }
                                        }
                                    }
                                    Ok(ts.to_owned())
                                }
                                Err(_) => Ok(ts),
                            }
                        }
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
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
                            let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                            let con = rb_
                                .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                                .await?;
                            let xcon: Arc<dyn Executor> = Arc::new(con);
                            ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                            xcon
                        };

                        let pass_args = match MxStoreService::invoke_pre_hook_(
                            full_uri.clone(),
                            dbs.0.select_hooks.clone(),
                            ctx.clone(),
                            local_args.to_vec(),
                        )
                        .await
                        {
                            Ok(t) => t,
                            Err(err) => {
                                ctx.lock().unwrap().set_failed();
                                return Err(err);
                            }
                        };

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
                                    if let Err(err) = MxStoreService::invoke_post_hook_(
                                        full_uri,
                                        dbs.0.select_hooks.clone(),
                                        ctx.clone(),
                                        pass_args,
                                    )
                                    .await
                                    {
                                        ctx.lock().unwrap().set_failed();
                                        return Err(err);
                                    }

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
                                                        log::info!("error for cache {err}");
                                                    }
                                                }
                                            }
                                            Ok(ts.to_owned())
                                        }
                                        Err(_) => Ok(ts),
                                    }
                                }
                                Err(err) => {
                                    ctx.lock().unwrap().set_failed();
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
                                ctx.lock().unwrap().set_failed();
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

        // let rb_ = mss.get_rbatis();
        let def_db_url = mss.get_db_url();

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
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let pass_args = match MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.query_hooks.clone(),
                        ctx.clone(),
                        local_args.clone(),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }
                    };
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
                                if let Err(err) = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.query_hooks.clone(),
                                    ctx.clone(),
                                    pass_args,
                                )
                                .await
                                {
                                    ctx.lock().unwrap().set_failed();
                                    return Err(err);
                                }

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
                                                    log::info!("Error for redis set {err}");
                                                }
                                            }
                                        }
                                        Ok(ts.to_owned())
                                    }
                                    Err(_) => Ok(ts),
                                }
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
                        },
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

        // let rb_ = mss.get_rbatis();
        let def_db_url = mss.get_db_url();

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
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let pass_args = match MxStoreService::invoke_pre_hook_(
                        full_uri.clone(),
                        dbs.0.query_hooks.clone(),
                        ctx.clone(),
                        local_args.clone(),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }
                    };

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
                                if let Err(err) = MxStoreService::invoke_post_hook_(
                                    full_uri,
                                    dbs.0.query_hooks.clone(),
                                    ctx.clone(),
                                    pass_args,
                                )
                                .await
                                {
                                    ctx.lock().unwrap().set_failed();
                                    return Err(err);
                                }

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
                                                    log::info!("Error for redis set {err}");
                                                }
                                            }
                                        }
                                        Ok(ts.to_owned())
                                    }
                                    Err(_) => Ok(ts),
                                }
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
                        },
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
        let hex_hash = format!("{hash:x}");
        format!("{}-{}-{}", uri.object.clone(), uri.method.clone(), hex_hash)
    }
}

impl Invocation for DbQueryServiceInvocation {
    fn invoke_return_option(
        &'static self,
        uri: &'_ InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        args: &'_ [Value],
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            let nspace = uri.namespace.clone();
            return Box::pin(async move { Err(anyhow!("Not found by NS {}.", nspace)) });
        };

        let dbs = if let Some(d) = mss.get_query(&uri.object) {
            DbQueryObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            let curi = uri.clone();
            return Box::pin(async move {
                Err(anyhow!(
                    "Not found by NS {}/{}.",
                    curi.namespace,
                    curi.object
                ))
            });
        };

        // mss.update_invocation_ctx(ctx);
        // mss.update_invocation_ctx_locked(ctx.clone());
        let method = uri.method.clone();
        let ns = uri.namespace.clone();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();

        log::info!("method_str: {method_str}");
        // let rb_ = if let Ok(r) = ctx.lock().unwrap().get_rbatis() {
        //     r.to_owned()
        // } else {
        //     mss.get_rbatis().to_owned()
        // };

        // let rb_ = mss.get_rbatis(); // .to_owned();
        let def_db_url = mss.get_db_url();

        let jwt = ctx
            .lock()
            .unwrap()
            .obtain_jwt_user_info()
            .unwrap_or(JwtUserClaims::anonymous());

        let conn_opt = ctx.lock().unwrap().get_rbatis_connection(&ns);

        match method_str {
            "find_one" => {
                let full_url = uri.url();
                let pass_args = args.to_vec();
                let cache_id = self.convert_to_cache_id(uri, &jwt, args);
                // log::info!("search {}", cache_id);
                Box::pin(async move {
                    if dbs.0.updatable {
                        return Err(anyhow!("Query was defined as updatable."));
                    }

                    if !dbs.0.onlyone {
                        return Err(anyhow!("Query was not defined as return one row."));
                    }

                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Option<Value>>(&cache_ret) {
                                return Ok(ret);
                            }
                        }
                    }

                    let conn = if let Some(conn) = conn_opt {
                        conn
                    } else {
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let mix_args = match MxStoreService::invoke_pre_hook_(
                        full_url.clone(),
                        dbs.0.hooks.clone(),
                        ctx.clone(),
                        pass_args.clone(),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }
                    };

                    let fix_arg = if !mix_args.is_empty() {
                        mix_args[0].to_owned()
                    } else {
                        pass_args[0].to_owned()
                    };
                    // let rb_ = mss.get_rbatis().to_owned();
                    if let Ok(qs) = dbs.to_condition(&pass_args) {
                        match dbs.query(conn, &jwt, &fix_arg, &qs).await {
                            Ok(rs) => {
                                // drop(rb_);
                                if !dbs.0.hooks.is_empty() {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", rs.clone());
                                    if let Err(err) = MxStoreService::invoke_post_hook_(
                                        full_url,
                                        dbs.0.hooks.clone(),
                                        ctx.clone(),
                                        pass_args,
                                    )
                                    .await
                                    {
                                        ctx.lock().unwrap().set_failed();
                                        return Err(err);
                                    }

                                    match ctx.lock().unwrap().get::<Vec<Value>>("RETURN_VALUE") {
                                        Ok(retval) => {
                                            let retvct = retval.first().map(|f| f.to_owned());
                                            if dbs.0.enable_cache {
                                                let tc = retvct.clone();
                                                if let Ok(text) = serde_json::to_string(&tc) {
                                                    // log::info!("set redis {} = {}", cache_id, text);
                                                    if let Err(err) = redis_set_expire(
                                                        &ns,
                                                        &cache_id,
                                                        &text,
                                                        dbs.0.cache_time.unwrap_or(30) as u64,
                                                    ) {
                                                        log::info!("Error for redis set {err}");
                                                    }
                                                }
                                            }
                                            Ok(retvct.to_owned())
                                        }
                                        Err(_) => {
                                            ctx.lock().unwrap().set_failed();
                                            Err(anyhow!("Could not GET/CAST the RETURN_VALUE"))
                                        }
                                    }
                                } else {
                                    let rtone = rs.first().map(|f| f.to_owned());
                                    if dbs.0.enable_cache {
                                        // let tc = rtone.clone();
                                        if let Ok(text) = serde_json::to_string(&rtone) {
                                            // log::info!("set redis {} = {}", cache_id, text);
                                            if let Err(err) = redis_set_expire(
                                                &ns,
                                                &cache_id,
                                                &text,
                                                dbs.0.cache_time.unwrap_or(30) as u64,
                                            ) {
                                                log::info!("Error for redis set {err}");
                                            }
                                        }
                                    }
                                    Ok(rtone)
                                }
                            }
                            Err(err) => {
                                // drop(rb_);
                                ctx.lock().unwrap().set_failed();
                                Err(err)
                            }
                        }
                    } else {
                        Ok(None)
                    }
                })
            }
            "execute" => {
                let full_url = uri.url();
                let pass_args = args.to_vec();
                let cache_id = self.convert_to_cache_id(uri, &jwt, args);
                // log::info!("search {}", cache_id);
                let tx_opt = ctx.lock().unwrap().get_tx_executor_sync(&ns);
                Box::pin(async move {
                    if !dbs.0.updatable {
                        return Err(anyhow!("Query was not updatable."));
                    }

                    if dbs.0.enable_cache {
                        if let Ok(Some(cache_ret)) = redis_get(&ns, &cache_id) {
                            if let Ok(ret) = serde_json::from_str::<Option<Value>>(&cache_ret) {
                                return Ok(ret);
                            }
                        }
                    }

                    let tx = if let Some(conn) = tx_opt {
                        conn
                    } else {
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let tcon = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let con_tx = tcon.begin().await?;
                        let txc = Arc::new(con_tx);

                        // let con = rb_.acquire_begin().await?;
                        // let txc = Arc::new(con);
                        ctx.lock().unwrap().set_tx_executor_sync(&ns, txc.clone());
                        txc
                    };

                    let mix_args = match MxStoreService::invoke_pre_hook_(
                        full_url.clone(),
                        dbs.0.hooks.clone(),
                        ctx.clone(),
                        pass_args.clone(),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }
                    };

                    let fix_arg = if !mix_args.is_empty() {
                        mix_args[0].to_owned()
                    } else {
                        pass_args[0].to_owned()
                    };
                    // let rb_ = mss.get_rbatis().to_owned();
                    if let Ok(qs) = dbs.to_condition(&pass_args) {
                        match dbs.execute(tx, &jwt, &fix_arg, &qs).await {
                            Ok(rs) => {
                                // drop(rb_);
                                if !dbs.0.hooks.is_empty() {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", rs.clone());
                                    if let Err(err) = MxStoreService::invoke_post_hook_(
                                        full_url,
                                        dbs.0.hooks.clone(),
                                        ctx.clone(),
                                        pass_args,
                                    )
                                    .await
                                    {
                                        ctx.lock().unwrap().set_failed();
                                        return Err(err);
                                    }

                                    match ctx.lock().unwrap().get::<Vec<Value>>("RETURN_VALUE") {
                                        Ok(retval) => {
                                            let retvct = retval.first().map(|f| f.to_owned());
                                            if dbs.0.enable_cache {
                                                let tc = retvct.clone();
                                                if let Ok(text) = serde_json::to_string(&tc) {
                                                    // log::info!("set redis {} = {}", cache_id, text);
                                                    if let Err(err) = redis_set_expire(
                                                        &ns,
                                                        &cache_id,
                                                        &text,
                                                        dbs.0.cache_time.unwrap_or(30) as u64,
                                                    ) {
                                                        log::info!("Error for redis set {err}");
                                                    }
                                                }
                                            }
                                            Ok(retvct.to_owned())
                                        }
                                        Err(_) => {
                                            ctx.lock().unwrap().set_failed();
                                            Err(anyhow!("Could not GET/CAST the RETURN_VALUE"))
                                        }
                                    }
                                } else {
                                    let rtone = rs.get(0).map(|f| f.to_owned());
                                    if dbs.0.enable_cache {
                                        // let tc = rtone.clone();
                                        if let Ok(text) = serde_json::to_string(&rtone) {
                                            // log::info!("set redis {} = {}", cache_id, text);
                                            if let Err(err) = redis_set_expire(
                                                &ns,
                                                &cache_id,
                                                &text,
                                                dbs.0.cache_time.unwrap_or(30) as u64,
                                            ) {
                                                log::info!("Error for redis set {err}");
                                            }
                                        }
                                    }
                                    Ok(rtone)
                                }
                            }
                            Err(err) => {
                                // drop(rb_);
                                ctx.lock().unwrap().set_failed();
                                Err(err)
                            }
                        }
                    } else {
                        Ok(None)
                    }
                })
            }
            _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
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

        // let rb_ = mss.get_rbatis(); // .to_owned();
        let def_db_url = mss.get_db_url();

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
                // log::info!("search {}", cache_id);
                Box::pin(async move {
                    if dbs.0.updatable {
                        ctx.lock().unwrap().set_failed();
                        return Err(anyhow!("Query was defined as updatable."));
                    }

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
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
                        let xcon: Arc<dyn Executor> = Arc::new(con);
                        ctx.lock().unwrap().set_rbatis_connection(&ns, xcon.clone());
                        xcon
                    };

                    let mix_args = match MxStoreService::invoke_pre_hook_(
                        full_url.clone(),
                        dbs.0.hooks.clone(),
                        ctx.clone(),
                        pass_args.clone(),
                    )
                    .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            ctx.lock().unwrap().set_failed();
                            return Err(err);
                        }
                    };

                    let fix_arg = if !mix_args.is_empty() {
                        mix_args[0].to_owned()
                    } else {
                        pass_args[0].to_owned()
                    };

                    // let rb_ = mss.get_rbatis().to_owned();
                    if let Ok(qs) = dbs.to_condition(&pass_args) {
                        match dbs.query(conn, &jwt, &fix_arg, &qs).await {
                            Ok(rs) => {
                                // drop(rb_);
                                if !dbs.0.hooks.is_empty() {
                                    ctx.lock().unwrap().insert("RETURN_VALUE", rs.clone());
                                    if let Err(err) = MxStoreService::invoke_post_hook_(
                                        full_url,
                                        dbs.0.hooks.clone(),
                                        ctx.clone(),
                                        pass_args,
                                    )
                                    .await
                                    {
                                        ctx.lock().unwrap().set_failed();
                                        return Err(err);
                                    }

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
                                                        log::info!("Error for redis set {err}");
                                                    }
                                                }
                                            }
                                            Ok(retval.to_owned())
                                        }
                                        Err(_) => {
                                            ctx.lock().unwrap().set_failed();
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
                                                log::info!("Error for redis set {err}");
                                            }
                                        }
                                    }
                                    Ok(rs)
                                }
                            }
                            Err(err) => {
                                // drop(rb_);
                                ctx.lock().unwrap().set_failed();
                                Err(err)
                            }
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
            ctx.lock().unwrap().set_failed();
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let dbs = if let Some(d) = mss.get_query(&uri.object) {
            DbQueryObject(d, mss.get_config(), AuthorizationConfig::get())
        } else {
            ctx.lock().unwrap().set_failed();
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let method = uri.method.clone();
        let ns = uri.namespace.clone();
        // let local_args = args.to_owned();
        // let mut ctx_ = ctx;
        let method_str = method.as_str();

        // let rb_ = mss.get_rbatis().to_owned();

        let def_db_url = mss.get_db_url();

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
                    if dbs.0.updatable {
                        ctx.lock().unwrap().set_failed();
                        return Err(anyhow!("Query was defined as updatable."));
                    }

                    if !dbs.0.pagable {
                        ctx.lock().unwrap().set_failed();
                        return Err(anyhow!("Query was not support paging search."));
                    }

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
                        let rb_ = get_multiple_rbatis_async(&def_db_url).await;
                        let con = rb_
                            .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                            .await?;
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
                    // let fix_arg = mix_args[0].to_owned();
                    let fix_arg = if !mix_args.is_empty() {
                        mix_args[0].to_owned()
                    } else {
                        pass_args[0].to_owned()
                    };

                    if let Ok(qs) = dbs.to_condition(&mix_args) {
                        let ret = dbs.paged_query(conn, &jwt, &fix_arg, &qs).await?;
                        if !dbs.0.hooks.is_empty() {
                            ctx.lock().unwrap().insert("RETURN_VALUE", ret.clone());
                            if let Err(err) = MxStoreService::invoke_post_hook_(
                                full_uri,
                                dbs.0.hooks.clone(),
                                ctx.clone(),
                                pass_args,
                            )
                            .await
                            {
                                ctx.lock().unwrap().set_failed();
                                return Err(err);
                            }

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
                                                log::info!("Error for redis set {err}");
                                            }
                                        }
                                    }
                                    Ok(retval.to_owned())
                                }
                                Err(_) => {
                                    ctx.lock().unwrap().set_failed();
                                    Err(anyhow!("Could not GET/CAST the RETURN_VALUE"))
                                }
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
                                        log::info!("Error for redis set {err}");
                                    }
                                }
                            }
                            Ok(ret)
                        }
                    } else {
                        Ok(Page::new_total(0, 10, 0))
                    }
                })
            }
            _ => Box::pin(async move { Err(anyhow!("Not implemented")) }),
        }
    }

    fn invoke_direct_query(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        query: String,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        self.invoke_direct_query_v2(uri, ctx, json!({"query": query}), args)
    }

    fn invoke_direct_query_v2(
        &'static self,
        uri: &InvokeUri,
        ctx: Arc<Mutex<InvocationContext>>,
        query: Value,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, Error>> + Send>> {
        let mss = if let Some(t) = MxStoreService::get(&uri.namespace) {
            t
        } else {
            ctx.lock().unwrap().set_failed();
            return Box::pin(async { Err(anyhow!("Not found by NS uri.namespace.")) });
        };

        let db_url = unwrap_json_string(&query, "db_url");
        let querybody = unwrap_json_string(&query, "query");

        let dbs = DbQueryObject(
            QueryObject::default(),
            mss.get_config(),
            AuthorizationConfig::get(),
        );

        let def_db_url = mss.get_db_url();

        let ns_ = uri.namespace.clone();
        Box::pin(async move {
            let (rb_, conn_opt) = if db_url.is_empty() {
                (
                    get_multiple_rbatis_async(&def_db_url).await,
                    ctx.lock().unwrap().get_rbatis_connection(&ns_),
                )
            } else {
                (get_multiple_rbatis_async(&db_url).await, None)
            };

            let conn = if let Some(conn) = conn_opt {
                conn
            } else {
                let con = rb_
                    .try_acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT))
                    .await?;
                let xcon: Arc<dyn Executor> = Arc::new(con);
                ctx.lock()
                    .unwrap()
                    .set_rbatis_connection(&ns_, xcon.clone());
                xcon
            };

            match dbs.direct_query(conn, &querybody, &args).await {
                Ok(t) => Ok(t),
                Err(err) => {
                    ctx.lock().unwrap().set_failed();
                    Err(err)
                }
            }
        })
    }
}
