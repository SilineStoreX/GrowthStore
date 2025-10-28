use anyhow::{anyhow, Result};
use rbatis::executor::{Executor, RBatisTxExecutor};
use salvo::jwt_auth::{JwtAuthDepotExt, JwtAuthState};
use salvo::Depot;
use substring::Substring;
use uuid::Uuid;

use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Mutex, OnceLock};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use crate::config::auth::JwtUserClaims;
use crate::pin_blockon_async_v2;
use crate::utils::redis::{redis_del, redis_lock_expire_nx};

type FnGenerateAuthorizationToken = fn(username: &str, user_id: &str, org: Option<String>) -> Result<String, anyhow::Error>;

static FUNC_GENERATE_AUTHORIZATION_TOKEN: OnceLock<FnGenerateAuthorizationToken> = OnceLock::new();

/**
 * 该函数只能调用一次，且必须在应用启动时调用
 */
pub fn setup_generate_authorization_token_func(newfunc: FnGenerateAuthorizationToken) {
    let _ = FUNC_GENERATE_AUTHORIZATION_TOKEN.get_or_init(|| newfunc);
}

pub fn proxy_generate_jwt_authorization_token(username: &str, user_id: &str, org: Option<String>) -> Result<String, anyhow::Error> {
    match FUNC_GENERATE_AUTHORIZATION_TOKEN.get() {
        Some(func) => {
            func(username, user_id, org)
        },
        None => {
            Err(anyhow!("generate_authorization_token func was not setup."))
        }
    }
}

pub struct InvocationContext {
    id: u64,
    map: HashMap<String, Box<dyn Any + Send + Sync>>,
    tx: BTreeMap<String, Arc<RBatisTxExecutor>>,
    // conn: HashMap<String, Arc<dyn Executor>>,
    conn: BTreeMap<String, Arc<dyn Executor>>,
    lockmap: HashMap<String, String>,
    success: bool,
}

unsafe impl Send for InvocationContext {}

unsafe impl Sync for InvocationContext {}

fn type_key<T: 'static>() -> String {
    format!("{:?}", TypeId::of::<T>())
}

impl Debug for InvocationContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InvocationContext")
            .field("id", &self.id)
            .finish()
    }
}

impl Default for InvocationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl InvocationContext {
    pub fn new() -> Self {
        Self {
            id: 0,
            map: HashMap::new(),
            tx: BTreeMap::new(),
            conn: BTreeMap::new(),
            lockmap: HashMap::new(),
            success: true,
        }
    }

    pub fn new_userclaims(jwt: JwtUserClaims) -> Self {
        let mut ctx_inner = InvocationContext::new();
        ctx_inner.inject(jwt);
        ctx_inner
    }

    pub fn new_with_id(id: u64) -> Self {
        Self {
            id,
            map: HashMap::new(),
            tx: BTreeMap::new(),
            conn: BTreeMap::new(),
            lockmap: HashMap::new(),
            success: true,
        }
    }

    pub fn arcnew() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_span_id(&self) -> Option<String> {
        self.get_string("__SPAN_ID")
    }

    pub fn set_span_id(&mut self, id: &str) {
        self.insert("__SPAN_ID", id.to_string());
    }

    /// Get reference to depot inner map.
    #[inline]
    pub fn inner(&self) -> &HashMap<String, Box<dyn Any + Send + Sync>> {
        &self.map
    }

    /// Creates an empty `Depot` with the specified capacity.
    ///
    /// The depot will be able to hold at least capacity elements without reallocating. If capacity is 0, the depot will not allocate.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            id: 0,
            map: HashMap::with_capacity(capacity),
            tx: BTreeMap::new(),
            conn: BTreeMap::new(),
            lockmap: HashMap::new(),
            success: true,
        }
    }
    /// Returns the number of elements the depot can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// Inject a value into the depot.
    #[inline]
    pub fn inject<V: Any + Send + Sync>(&mut self, value: V) -> &mut Self {
        self.map.insert(type_key::<V>(), Box::new(value));
        self
    }

    /// Obtain a reference to a value previous inject to the depot.
    ///
    /// Returns `Err(None)` if value is not present in depot.
    /// Returns `Err(Some(Box<dyn Any + Send + Sync>))` if value is present in depot but downcast failed.
    #[inline]
    pub fn obtain<T: Any + Send + Sync>(&self) -> Result<&T, Option<&Box<dyn Any + Send + Sync>>> {
        self.get(&type_key::<T>())
    }

    #[inline]
    pub fn obtain_jwt_user_info(&self) -> Option<JwtUserClaims> {
        match self.obtain::<JwtUserClaims>() {
            Ok(t) => Some(t.clone()),
            Err(_) => Some(JwtUserClaims::anonymous()),
        }
    }

    pub fn get_domain(&self) -> Option<String> {
        self.obtain_jwt_user_info().map(|s| s.domain)
    }

    pub fn set_current_user(&mut self, username: &str, userid: Option<&str>) {
        self.inject(JwtUserClaims::username_with_id(username, userid.unwrap_or_default()));
    }

    pub fn get_current_user(&self) -> Option<String> {
        self.obtain_jwt_user_info().map(|j| j.username)
    }

    pub fn get_current_userid(&self) -> Option<String> {
        self.obtain_jwt_user_info().map(|j| j.userid)
    }


    pub fn get_authorization(&self, username: &str, userid: &str, org: Option<String>) -> Option<String> {
        proxy_generate_jwt_authorization_token(username, userid, org).map(Some).unwrap_or(None)
    }

    pub fn get_current_authorization(&self) -> Option<String> {
        let curr_username = self.get_current_user().unwrap_or("".to_string());
        let curr_userid = self.get_current_userid().unwrap_or("0".to_string());
        proxy_generate_jwt_authorization_token(&curr_username, &curr_userid, None).map(Some).unwrap_or(None)
    }

    /// Obtain a mutable reference to a value previous inject to the depot.
    ///
    /// Returns `Err(None)` if value is not present in depot.
    /// Returns `Err(Some(Box<dyn Any + Send + Sync>))` if value is present in depot but downcast failed.
    #[inline]
    pub fn obtain_mut<T: Any + Send + Sync>(
        &mut self,
    ) -> Result<&mut T, Option<&mut Box<dyn Any + Send + Sync>>> {
        self.get_mut(&type_key::<T>())
    }

    /// Inserts a key-value pair into the depot.
    #[inline]
    pub fn insert<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Any + Send + Sync,
    {
        self.map.insert(key.into(), Box::new(value));
        self
    }

    /// Check is there a value stored in depot with this key.
    #[inline]
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
    /// Check is there a value is injected to the depot.
    ///
    /// **Note: This is only check injected value.**
    #[inline]
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.map.contains_key(&type_key::<T>())
    }

    /// Immutably borrows value from depot.
    ///
    /// Returns `Err(None)` if value is not present in depot.
    /// Returns `Err(Some(Box<dyn Any + Send + Sync>))` if value is present in depot but downcast failed.
    #[inline]
    pub fn get<V: Any + Send + Sync>(
        &self,
        key: &str,
    ) -> Result<&V, Option<&Box<dyn Any + Send + Sync>>> {
        if let Some(value) = self.map.get(key) {
            value.downcast_ref::<V>().ok_or(Some(value))
        } else {
            Err(None)
        }
    }

    #[inline]
    pub fn get_(&self, key: &str) -> Option<&Box<dyn Any + Send + Sync>> {
        self.map.get(key)
    }

    #[inline]
    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.get::<String>(key) {
            Ok(t) => Some(t.clone()),
            Err(_) => None,
        }
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> bool {
        match self.get::<bool>(key) {
            Ok(t) => *t,
            Err(_) => false,
        }
    }

    #[inline]
    pub fn get_i64(&self, key: &str) -> i64 {
        match self.get::<i64>(key) {
            Ok(t) => *t,
            Err(_) => 0i64,
        }
    }

    #[inline]
    pub fn get_status(&self) -> i64 {
        self.get_i64("RETURN_STATUS")
    }

    #[inline]
    pub fn get_message(&self) -> String {
        self.get_string("RETURN_MESSAGE")
            .unwrap_or("SUCCESS".to_owned())
    }

    #[inline]
    pub fn set_status(&mut self, st: i64) {
        self.insert("RETURN_STATUS", st);
    }

    #[inline]
    pub fn set_message(&mut self, msg: &str) {
        self.insert("RETURN_MESSAGE", msg.to_string());
    }

    #[inline]
    pub fn set_return_rawdata(&mut self, cust: bool) {
        let custom = if cust { 1i64 } else { 0i64 };
        self.insert("CUSTOM_RETURN_RAWDATA", custom);
    }

    #[inline]
    pub fn get_return_rawdata(&self) -> bool {
        self.get_i64("CUSTOM_RETURN_RAWDATA") == 1i64
    }

    #[inline]
    pub fn set_response_xml(&mut self, cust: bool) {
        let custom = if cust { 1i64 } else { 0i64 };
        self.insert("CUSTOM_RESPONSE_XML", custom);
    }

    #[inline]
    pub fn get_response_xml(&self) -> bool {
        self.get_i64("CUSTOM_RESPONSE_XML") == 1i64
    }

    #[inline]
    pub fn get_u64(&self, key: &str) -> u64 {
        match self.get::<u64>(key) {
            Ok(t) => *t,
            Err(_) => 0u64,
        }
    }

    /// Mutably borrows value from depot.
    ///
    /// Returns `Err(None)` if value is not present in depot.
    /// Returns `Err(Some(Box<dyn Any + Send + Sync>))` if value is present in depot but downcast failed.
    pub fn get_mut<V: Any + Send + Sync>(
        &mut self,
        key: &str,
    ) -> Result<&mut V, Option<&mut Box<dyn Any + Send + Sync>>> {
        if let Some(value) = self.map.get_mut(key) {
            if value.downcast_mut::<V>().is_some() {
                Ok(value
                    .downcast_mut::<V>()
                    .expect("downcast_mut shuold not be failed"))
            } else {
                Err(Some(value))
            }
        } else {
            Err(None)
        }
    }

    /// Remove value from depot and returning the value at the key if the key was previously in the depot.
    #[inline]
    pub fn remove<V: Any + Send + Sync>(
        &mut self,
        key: &str,
    ) -> Result<V, Option<Box<dyn Any + Send + Sync>>> {
        if let Some(value) = self.map.remove(key) {
            value.downcast::<V>().map(|b| *b).map_err(Some)
        } else {
            Err(None)
        }
    }

    /// Delete the key from depot, if the key is not present, return `false`.
    #[inline]
    pub fn delete(&mut self, key: &str) -> bool {
        self.map.remove(key).is_some()
    }

    /// Remove value from depot and returning the value if the type was previously in the depot.
    #[inline]
    pub fn scrape<T: Any + Send + Sync>(
        &mut self,
    ) -> Result<T, Option<Box<dyn Any + Send + Sync>>> {
        self.remove(&type_key::<T>())
    }

    pub fn get_tx_executor_sync(&mut self, ns: &str) -> Option<Arc<RBatisTxExecutor>> {
        self.tx.get(ns).cloned()
    }

    pub fn set_tx_executor_sync(&mut self, ns: &str, tx: Arc<RBatisTxExecutor>) {
        self.tx.insert(ns.to_string(), tx.clone());

        if let Some(cn) = self.conn.insert(ns.to_string(), tx) {
            drop(cn);
        }
    }

    pub fn set_rbatis_connection(&mut self, ns: &str, conn: Arc<dyn Executor>) {
        self.conn.insert(ns.to_string(), conn);
    }

    pub fn get_rbatis_connection(&self, ns: &str) -> Option<Arc<dyn Executor>> {
        self.conn.get(ns).cloned()
    }

    pub fn set_failed(&mut self) {
        self.success = false;
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn get_locks(&self) -> HashMap<String, String> {
        self.lockmap.clone()
    }

    pub fn lock(
        &mut self,
        ns: &str,
        key: &str,
        expire: i64,
    ) -> Result<Option<String>, anyhow::Error> {
        let _ = redis_lock_expire_nx(
            ns,
            key,
            &Uuid::new_v4().to_string(),
            expire as u64,
            Some(true),
        )?;
        self.lockmap.insert(format!("{ns}-{key}"), ns.to_owned());
        Ok(None)
    }

    pub fn unlock(&mut self, ns: &str, key: &str) -> Result<Option<String>, anyhow::Error> {
        let _ = redis_del(ns, key)?;
        self.lockmap.remove(&format!("{ns}-{key}"));
        Ok(None)
    }

    fn release_locks(&mut self) {
        for (k, v) in self.lockmap.iter() {
            let key = k.substring(v.len() + 1, k.len());
            let ns = v.clone();
            let _ = redis_del(&ns, key);
        }
        self.lockmap.clear();
    }

    pub async fn commit_or_rollback(&mut self, commit: bool) -> Result<(), anyhow::Error> {
        let mut should_commit = commit;
        let mut ret_err = None;
        for (ns, tx_) in &self.tx {
            if should_commit {
                log::info!("execute commit on conn directly {ns}.");
                if let Err(err) = tx_.conn.lock().await.commit().await {
                    log::info!("error for commit {err}");
                    ret_err = Some(err);
                    should_commit = false;
                }
            } else {
                log::info!("execute rollback on conn directly {ns}.");
                if let Err(err) = tx_.conn.lock().await.rollback().await {
                    log::info!("error for rollback {err}");
                    ret_err = Some(err);
                    should_commit = false;
                }
            }
        }

        self.tx.clear();

        if ret_err.is_none() {
            Ok(())
        } else {
            Err(anyhow!(ret_err.unwrap()))
        }
    }

    pub fn finalize_async(&mut self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let tx_ = self.tx.clone();
        let commit = self.success;
        self.conn.clear();
        self.tx.clear();
        Box::pin(async move {
            if let Err(err) = context_finalize(&tx_, commit).await {
                log::info!("error for commit or rollback {err}");
            }
        })
    }

    pub async fn finalize_async_v2(&mut self) {
        let tx_ = self.tx.clone();
        let commit = self.success;
        self.finalize();

        if let Err(err) = context_finalize(&tx_, commit).await {
            log::info!("error for commit or rollback {err}");
        }
    }

    fn finalize(&mut self) {
        self.tx.clear();
        self.map.clear();
        // self.conn.clear();
    }
}

async fn context_finalize(
    tx: &BTreeMap<String, Arc<RBatisTxExecutor>>,
    commit: bool,
) -> Result<(), anyhow::Error> {
    let mut should_commit = commit;
    let mut ret_err = None;
    for (ns_, tx_) in tx {
        log::debug!("commit({should_commit}) or rollback for {ns_}");
        if should_commit {
            // log::info!("execute commit on conn {ns}.");
            if let Err(err) = tx_.conn.lock().await.commit().await {
                log::warn!("error for commit {err}");
                ret_err = Some(err);
                should_commit = false;
            }
        } else {
            // log::info!("execute rollback on conn {ns}.");
            if let Err(err) = tx_.conn.lock().await.rollback().await {
                log::warn!("error for rollback {err}");
                ret_err = Some(err);
                should_commit = false;
            }
        }
        // drop(tx_);
    }

    if ret_err.is_none() {
        Ok(())
    } else {
        Err(anyhow!(ret_err.unwrap()))
    }
}

pub async fn release_all_connections(ictx: &Arc<std::sync::Mutex<InvocationContext>>) {
    let (tx_, commit) = if let Ok(mut ctx) = ictx.lock() {
        let tx_ = ctx.tx.clone();
        let succ = ctx.success;
        ctx.conn.clear();
        ctx.tx.clear();
        (tx_, succ)
    } else {
        return;
    };

    log::debug!("Release Ctx by commit({commit}).");

    if let Err(err) = context_finalize(&tx_, commit).await {
        log::warn!("could not finalized the context with error {err}");
    }
}

pub async fn release_context(ictx: &Arc<std::sync::Mutex<InvocationContext>>) {
    let (tx_, commit) = if let Ok(mut ctx) = ictx.lock() {
        let tx_ = ctx.tx.clone();
        let succ = ctx.success;
        ctx.tx.clear();
        (tx_, succ)
    } else {
        return;
    };

    log::debug!("Release Ctx by commit({commit}).");

    if let Err(err) = context_finalize(&tx_, commit).await {
        log::warn!("could not finalized the context with error {err}");
    }
}

impl Drop for InvocationContext {
    fn drop(&mut self) {
        // log::info!("InvocationContext dropped.");

        let tx_ = self.tx.clone();
        self.tx.clear();
        if !tx_.is_empty() {
            let commit = self.success;
            let _ = pin_blockon_async_v2!(async move {
                if let Err(err) = context_finalize(&tx_, commit).await {
                    log::info!("error on finallize {err}");
                }
                // Box(0) as Box<dyn Any + Send + Sync>
            })
            .is_ok();
        }

        self.release_locks();
        // self.finalize();
        // self.conn.drain();
    }
}

pub trait JwtFromDepot {
    fn from_depot(depot: &mut Depot) -> Self;
}

impl JwtFromDepot for InvocationContext {
    fn from_depot(depot: &mut Depot) -> Self {
        let mut ctx_inner = InvocationContext::new();
        if depot.jwt_auth_state() == JwtAuthState::Authorized {
            if let Some(jwtdata) = depot.jwt_auth_data::<JwtUserClaims>() {
                ctx_inner.inject(jwtdata.claims.clone());
            }
        }

        if let Ok(spid) = depot.get::<String>("__SPAN_ID") {
            ctx_inner.set_span_id(&spid);
        }
        ctx_inner
    }
}
