use anyhow::{anyhow, Result};
use rbatis::executor::{Executor, RBatisTxExecutor};
use salvo::jwt_auth::{JwtAuthDepotExt, JwtAuthState};
use salvo::Depot;

use std::{
    any::{Any, TypeId}, collections::HashMap, mem::MaybeUninit, sync::{atomic::AtomicU64, Arc, Once}
};

use crate::pin_blockon_async;
use crate::config::auth::JwtUserClaims;

static INVOCATION_CTX_ID_REF: AtomicU64 = AtomicU64::new(1);

pub struct InvocationContext {
    id: u64,
    map: HashMap<String, Box<dyn Any + Send + Sync>>,
    tx: HashMap<String, Arc<RBatisTxExecutor>>,
    conn: HashMap<String, Arc<dyn Executor>>,
    success: bool,
}

unsafe impl Send for InvocationContext {}

unsafe impl Sync for InvocationContext {}

fn type_key<T: 'static>() -> String {
    format!("{:?}", TypeId::of::<T>())
}

impl Default for InvocationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl InvocationContext {
    pub fn get_static_hashmap() -> &'static mut HashMap<u64, InvocationContext> {
        static mut INVOCATION_CTX_HOLDER: MaybeUninit<HashMap<u64, InvocationContext>> =
            MaybeUninit::uninit();
        // Once带锁保证只进行一次初始化
        static INVOCATION_CTX_HOLDER_ONCE: Once = Once::new();

        INVOCATION_CTX_HOLDER_ONCE.call_once(|| unsafe {
            INVOCATION_CTX_HOLDER.as_mut_ptr().write(HashMap::new());
        });

        unsafe { &mut (*INVOCATION_CTX_HOLDER.as_mut_ptr()) }
    }

    pub fn get_static() -> &'static mut Self {
        let n = Self::new_with_id(
            INVOCATION_CTX_ID_REF.fetch_add(1, std::sync::atomic::Ordering::Acquire),
        );
        let id = n.id;
        Self::get_static_hashmap().insert(n.id, n);
        Self::get_static_hashmap().get_mut(&id).unwrap()
    }

    pub fn get_static_id(id: u64) -> &'static mut Self {
        Self::get_static_hashmap().get_mut(&id).unwrap()
    }

    pub fn remove_static_id(id: u64) {
        Self::get_static_hashmap().remove(&id);
    }

    pub async fn finalizeby_id(id: u64) {
        if id > 0 {
            if let Some(t) = Self::get_static_hashmap().get_mut(&id) {
                log::info!("executing commit or rollback;");
                if let Err(err) = t.commit_or_rollback(true).await {
                    log::info!("Error on transaction commit {}", err);
                }
            }
            Self::remove_static_id(id);
        }
    }

    pub fn new() -> Self {
        Self {
            id: 0,
            map: HashMap::new(),
            tx: HashMap::new(),
            conn: HashMap::new(),
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
            tx: HashMap::new(),
            conn: HashMap::new(),
            success: true,
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
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
            tx: HashMap::new(),
            conn: HashMap::new(),
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
        self.tx.get(ns).map(|f| f.to_owned())
    }

    pub fn set_tx_executor_sync(&mut self, ns: &str, tx: Arc<RBatisTxExecutor>) {
        self.tx.insert(ns.to_string(), tx);
    }

    pub fn set_rbatis_connection(&mut self, ns: &str, conn: Arc<dyn Executor>) {
        self.conn.insert(ns.to_string(), conn);
    }

    pub fn get_rbatis_connection(&self, ns: &str) -> Option<Arc<dyn Executor>> {
        self.conn.get(ns).map(|f| f.to_owned())
    }

    pub fn set_failed(&mut self) {
        self.success = false;
    }

    pub fn is_success(&self) -> bool {
        self.success
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

    pub async fn finalize_async(&mut self) {
        if let Err(err) = self.commit_or_rollback(self.success).await {
            log::info!("error for commit or rollback {err}");
        }
        self.finalize();
    }

    fn finalize(&mut self) {
        let id = self.id;
        self.tx.clear();
        self.map.clear();
        self.conn.clear();
        
        if id > 0 {
            Self::remove_static_id(id);
        }
    }
}

async fn context_finalize(tx: HashMap<String, Arc<RBatisTxExecutor>>, commit: bool) -> Result<(), anyhow::Error> {
    let mut should_commit = commit;
    let mut ret_err = None;
    for (ns, tx_) in &tx {
        if should_commit {
            log::info!("execute commit on conn {ns}.");
            if let Err(err) = tx_.conn.lock().await.commit().await {
                log::info!("error for commit {err}");
                ret_err = Some(err);
                should_commit = false;
            }
        } else {
            log::info!("execute rollback on conn {ns}.");
            if let Err(err) = tx_.conn.lock().await.rollback().await {
                log::info!("error for rollback {err}");
                ret_err = Some(err);
                should_commit = false;
            }
        }
    }

    if ret_err.is_none() {
        Ok(())
    } else {
        Err(anyhow!(ret_err.unwrap()))
    }
}

impl Drop for InvocationContext {
    fn drop(&mut self) {
        log::info!("InvocationContext dropped.");
        let tx_ = self.tx.clone();
        let commit = self.success;
        let _ = pin_blockon_async!(async move {
            if let Err(err) = context_finalize(tx_, commit).await {
                log::info!("error on finallize {err}");
            }
            Box::new(0) as Box<dyn Any + Send + Sync>
        }).unwrap_or(0);
        self.finalize();
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
        ctx_inner
    }
}
