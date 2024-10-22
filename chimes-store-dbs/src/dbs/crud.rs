use std::sync::Arc;
use anyhow::{anyhow, Error};
use chimes_dbs_factory::get_insert_field_value_present;
use chimes_dbs_factory::get_update_field_value_present;
use chimes_store_core::config::auth::AuthorizationConfig;
use chimes_store_core::config::auth::JwtUserClaims;
use chimes_store_core::config::ConditionItem;
use chimes_store_core::config::StoreObject;
use chimes_store_core::config::StoreServiceConfig;
use chimes_store_core::utils::global_data::copy_value_compared_replaced;
use chimes_store_core::utils::global_data::copy_value_excluded;
use chimes_store_core::utils::global_data::copy_value_replaced;
use itertools::Itertools;
use rbatis::executor::Executor;
use rbatis::rbdc::Uuid;
use rbatis::{executor::RBatisTxExecutor, IPageRequest, Page};
use serde_json::json;
use serde_json::Value;

use chimes_store_core::config::QueryCondition;

use crate::dbs::decode_vec_custom_fields_list;

use super::crypto_desenstize_process;
use super::is_desensitize_with_crypto_store;
use super::refine_column_value;
use super::refine_column_value_option;

pub(crate) trait DbCrud<T: Sized + Send + Sync> {
    async fn insert(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &T,
    ) -> Result<T, Error>;
    async fn update(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &T,
    ) -> Result<T, Error>;
    async fn delete(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &T,
    ) -> Result<T, Error>;
    async fn delete_by(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Value, Error>;
    async fn upsert(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: &Value,
        qs: Option<QueryCondition>,        
    ) -> Result<Value, Error>;

    #[allow(dead_code)]
    async fn save_batch(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: Vec<Value>        
    ) -> Result<Value, Error>;    
    async fn update_by(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: &Value,
        qs: &QueryCondition,
    ) -> Result<Value, Error>;

    async fn select(&self, rb: Arc<dyn Executor>, jwt: &JwtUserClaims, t: &T) -> Result<Option<T>, Error>;
    async fn find_one(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Option<T>, Error>;
    async fn query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Vec<T>, Error>;
    async fn paged_query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Page<T>, Error>;    
}

pub fn validate_object(val: &Value, st: &StoreObject, pkcheck: bool) -> Result<(), anyhow::Error> {
    if val.is_object() {
        if let Some(ts) = val.as_object() {
            if ts.is_empty() {
                Err(anyhow!("Object does not contains any fields."))
            } else {
                // later we will check the field to validate the format
                if pkcheck {
                    let keys = st.get_key_columns();
                    for key in keys {
                        let keyname = key.prop_name.unwrap_or(key.field_name);
                        if !ts.contains_key(&keyname) {
                            return Err(anyhow!("No primary key Property {keyname} present",));
                        }
                    }
                }
                Ok(())
            }
        } else {
            Err(anyhow!("No object present"))
        }
    } else {
        Err(anyhow!("No object present"))
    }
}

//
// 对于Column中定义了Relation且RelationArray为True的列
// 则是按照主表中的主键进行查询，因为这是一种1..N的关系，在主表中，是没有保存附属表的ID的，通常不会在主表中保存附属表的ID数组。
// 因此，在加载RelationArray时，需要往Vec<Column>中加入一条Column的定义，col_type指定其为Relation，字段名为主键ID，属性名可以自行定义
// 加载后，所生成的JSON，就会包含有以属性名定义JSON Array，其为附属表的内容
// 一个StoreObject中可以允许定义多个附属表的关联对象
// 而对于对于Column中定义了Relation且RelationArray为False的列
// 则对应的该字段所保存的是附属表的主键ID，则直接已该字段的值对Relation Object进行查询即可
// 级联更新/新增，只能支持一个层级
// json!{
//    val: json!{
//         val2: json!{ ... }
//    }
// }
// 上面的JSON中，val2将不被支持了
//
pub struct DbStoreObject(
    pub StoreObject,
    pub StoreServiceConfig,
    pub AuthorizationConfig,
);

async fn insert_casc_2(
    _sto: &StoreObject,
    _stc: &StoreServiceConfig,
    _auth: &AuthorizationConfig,
    _executor: Arc<RBatisTxExecutor>,
    _jwt: &JwtUserClaims,
    _t: &Value,
    _isarray: bool,
) -> Result<Value, anyhow::Error> {
    Ok(Value::Null)
}

async fn insert_casc_1(
    sto: &StoreObject,
    stc: &StoreServiceConfig,
    auth: &AuthorizationConfig,
    executor: Arc<RBatisTxExecutor>,
    jwt: &JwtUserClaims,
    t: &Value,
    isarray: bool,
) -> Result<Value, anyhow::Error> {
    let mut update_map = serde_json::Map::new();
    for col in sto
        .fields
        .iter()
        .filter(|p| p.col_type == Some("relation".to_string()) && isarray == p.relation_array)
        .cloned()
    {
        if let Some(vt) = t.get(col.prop_name.clone().unwrap_or(col.field_name.clone())) {
            if let Some(sto) = stc.get_object(&col.relation_object.clone().unwrap_or_default()) {
                let dbx = Box::new(DbStoreObject(sto.to_owned(), stc.clone(), auth.clone()));
                match vt {
                    Value::Object(ts) => {
                        // if this is a json object, it should be a object to be update
                        log::info!("TS: {:?}", ts);
                        if let Some(mt) =
                            ts.get(&col.relation_field.clone().unwrap_or("id".to_owned()))
                        {
                            if mt.is_null() {
                                // insert, the new id should be update to major table
                                match dbx.insert_(executor.clone(), jwt, vt).await {
                                    Ok(tret) => {
                                        log::info!("insert_: {:?}", tret);
                                        update_map.insert(
                                            col.prop_name.clone().unwrap_or(col.field_name.clone()),
                                            tret,
                                        );
                                    }
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            } else {
                                // do update
                                match dbx.update_(executor.clone(), jwt, vt).await {
                                    Ok(_) => {
                                        // update_map.insert("k", tret.get("last_insert_id").map(|f| f.to_owned()).unwrap_or(Value::Null));
                                    }
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            }
                        } else {
                            // insert, new id should be update to major table
                            match dbx.insert_(executor.clone(), jwt, vt).await {
                                Ok(tret) => {
                                    log::info!("insert_: {:?}", tret);
                                    update_map.insert(
                                        col.prop_name.clone().unwrap_or(col.field_name.clone()),
                                        tret,
                                    );
                                }
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        }
                    }
                    Value::Array(list) => {
                        log::info!("Array List: {:?}", list);
                        // If there is a list belone to relationship, we should choose delete first or update only
                        // So that, we will check this
                        if let Some(stm) =
                            stc.get_object(&col.relation_object.clone().unwrap_or_default())
                        {
                            let dbx =
                                Box::new(DbStoreObject(stm.to_owned(), stc.clone(), auth.clone()));
                            if col.relation_array {
                                if let Some(tvl) = t.get(col.field_name.clone()) {
                                    let mut del_qs = QueryCondition::default();
                                    del_qs.and.push(ConditionItem {
                                        field: col.relation_field.clone().unwrap_or_default(),
                                        op: "=".to_owned(),
                                        value: tvl.clone(),
                                        ..Default::default()
                                    });
                                    let pkey = dbx.get_primary_key_field();
                                    if pkey.is_some() {
                                        let pkey_str = pkey.unwrap();
                                        let mut mst = vec![];
                                        for vl in list {
                                            if let Some(kv) = vl.get(&pkey_str) {
                                                mst.push(kv.clone());
                                            }
                                        }

                                        if !mst.is_empty() {
                                            del_qs.and.push(ConditionItem {
                                                field: dbx
                                                    .get_primary_key_field()
                                                    .unwrap_or_default(),
                                                op: " NOT IN ".to_owned(),
                                                value: Value::Array(mst),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                    let _ = dbx.delete_by(executor.clone(), jwt, &del_qs).await?;
                                }
                            }
                            for val in list {
                                // replace the val of to
                                let mut mutval = val.clone();
                                if let Value::Object(mp) = &mut mutval {
                                    // 备注：如果是1..N关联，则需要在表数据定义中产生一个该字段field_name与prop_name保持一致的Column，不然，会找不到？
                                    if let Some(tvl) = t.get(col.field_name.clone()) {
                                        mp.insert(
                                            col.relation_field.clone().unwrap_or_default(),
                                            tvl.clone(),
                                        );
                                    }
                                };
                                match dbx.upsert_(executor.clone(), jwt, &mutval, None).await {
                                    Ok(_) => {}
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        // do nothing
                    }
                }
            }
        }
    }
    Ok(Value::Object(update_map))
}

impl DbStoreObject {
    /**
     * 获取Key的对应值
     */
    pub(crate) fn get_keys_values(&self, t: &Value) -> Vec<rbs::Value> {
        let mut answers = vec![];
        if t.is_object() {
            for k in self.0.get_key_columns() {
                let prop = k.prop_name.clone().unwrap_or(k.field_name.clone());
                let v = refine_column_value_option(&t.get(&prop), &k);
                answers.push(rbs::to_value!(v));
            }
        } else {
            for k in self.0.get_key_columns() {
                let v = refine_column_value(t, &k);
                answers.push(rbs::to_value!(v));
            }
        }
        answers
    }

    /**
     * 获得pkey的字段名称
     */
    pub fn get_primary_key_field(&self) -> Option<String> {
        if self.0.get_key_columns().is_empty() {
            None
        } else {
            self.0.get_key_columns().first().map(|f| f.field_name.clone())
        }
    }

    pub fn get_pkey_value_present(&self, t: &Value) -> Option<String> {
        let keys = self.0.get_key_columns();
        if keys.is_empty() {
            None
        } else {
            let mut mppt = vec![];
            for cl in keys {
                let prop = cl.prop_name.unwrap_or(cl.field_name);
                let val = if t.is_object() {
                    t.get(&prop).map(|v| v.to_owned()).unwrap_or(Value::Null)
                } else {
                    t.to_owned()
                };
                if let Some(tid) = val.as_str() {
                    mppt.push(format!("{}={}", prop, tid));
                } else {
                    mppt.push(format!("{}={}", prop, val));
                }
            }
            Some(mppt.join("&").to_string())
        }
    }

    pub(crate) fn get_field_value(&self, field: &str, t: &Value) -> Value {
        if t.is_object() {
            match t.get(field) {
                Some(v) => v.clone(),
                None => Value::Null,
            }
        } else {
            t.clone()
        }
    }

    pub (crate) fn get_generated_value(&self, generater: &str, jwt: &JwtUserClaims) -> rbs::Value {
        
        match generater {
            "autoincrement" => rbs::to_value!(Value::Null),
            "snowflakeid" => rbs::to_value!(rbatis::snowflake::new_snowflake_id()),
            "uuid" => rbs::to_value!(Uuid::new()),
            "cur_user_id" => rbs::to_value!(jwt.userid.clone()),
            "cur_user_name" => rbs::to_value!(jwt.username.clone()),
            "cur_datetime" => rbs::to_value!(rbatis::rbdc::DateTime::now()),
            "cur_date" => rbs::to_value!(rbatis::rbdc::DateTime::now()),
            "cur_time" => rbs::to_value!(rbatis::rbdc::DateTime::now()),
            "mod_user_id" => rbs::to_value!(jwt.userid.clone()),
            "mod_user_name" => rbs::to_value!(jwt.username.clone()),
            "mod_datetime" => rbs::to_value!(rbatis::rbdc::DateTime::now()),
            "mod_date" => rbs::to_value!(rbatis::rbdc::DateTime::now()),
            "mod_time" => rbs::to_value!(rbatis::rbdc::DateTime::now()),
            _ => rbs::to_value!(Value::Null)
        }
    }

    pub(crate) fn to_insert_rbs_value_vec(
        &self,
        rb: &rbatis::RBatis,
        jwt: &JwtUserClaims,
        t: &Value,
        mp: &Value,
    ) -> (Vec<rbs::Value>, String, String) {
        let mut answers = vec![];
        let mut insert_fields = vec![];
        let mut insert_values = vec![];
        let ns = self.1.namespace.clone();
        // let mut pkeys = vec![];

        for col in self.0.fields.iter() {
            let prop = col.prop_name.clone().unwrap_or(col.field_name.clone());
            if col.col_type != Some("relation".to_owned()) {
                // insert_values.push("?");
                let generator = col.generator.clone().unwrap_or_default();
                if generator != *"autoincrement" {
                    insert_fields.push(col.field_name.clone().to_string());
                    insert_values.push(get_insert_field_value_present(rb, &col.field_name.clone().to_string(), &col.field_type.clone().unwrap_or_default()));
                    if let Some(tv) = t.get(&prop) {
                        if tv.is_null() {
                            if let Some(generater) = col.generator.clone() {
                                answers.push(self.get_generated_value(&generater, jwt));
                            } else {
                                answers.push(rbs::to_value!(tv));
                            }
                        } else if is_desensitize_with_crypto_store(&col.desensitize, col.crypto_store) {
                            let text = tv.as_str().unwrap_or_default();
                            let crypto_text = crypto_desenstize_process(text.to_owned(), &ns, &col.desensitize);
                            answers.push(rbs::to_value!(crypto_text));
                        } else {
                            answers.push(rbs::to_value!(tv));
                        }
                    } else if let Some(generater) = col.generator.clone() {
                        answers.push(self.get_generated_value(&generater, jwt));
                    } else {
                        answers.push(rbs::to_value!(Value::Null));
                    }
                }
            } else if !col.relation_array {
                insert_fields.push(col.field_name.clone().to_string());
                // insert_values.push("?");
                insert_values.push(get_insert_field_value_present(rb, &col.field_name.clone().to_string(), &col.field_type.clone().unwrap_or_default()));
                // 两种情况，一种是mp中有新的值，另一种是t中有这个值
                if let Some(smv) = mp.get(&prop) {
                    let relprop = col.relation_field.clone().unwrap_or_default();
                    if let Some(relp) = smv.get(&relprop) {
                        answers.push(rbs::to_value!(relp));
                    } else {
                        answers.push(rbs::to_value!(smv));
                    }
                } else if let Some(smv) = t.get(&prop) {
                    let relprop = col.relation_field.clone().unwrap_or_default();
                    log::info!("Relp: {}", relprop);
                    if let Some(relp) = smv.get(&relprop) {
                        answers.push(rbs::to_value!(relp));
                    } else {
                        answers.push(rbs::to_value!(smv));
                    }
                } else {
                    answers.push(rbs::to_value!(Value::Null));
                }
            }
        }

        (answers, insert_fields.join(","), insert_values.join(","))
    }

    pub(crate) fn to_update_rbs_value_vec(
        &self,
        rb: &rbatis::RBatis,
        jwt: &JwtUserClaims,
        t: &Value,
        with_key: bool,
    ) -> (Vec<rbs::Value>, String, String) {
        let mut answers = vec![];
        let mut answers_keys = vec![];
        let mut update_fields = vec![];
        let mut key_fields = vec![];
        let ns = self.1.namespace.clone();
        // let mut pkeys = vec![];

        for col in self.0.fields.iter() {
            let prop = col.prop_name.clone().unwrap_or(col.field_name.clone());
            if let Some(v) = t.get(prop) {
                if col.pkey {
                    answers_keys.push(rbs::to_value!(v.to_owned()));                    
                    if with_key {
                        key_fields.push(get_update_field_value_present(rb, &col.field_name.clone(), &col.field_type.clone().unwrap_or_default()));
                        // key_fields.push(format!("{} = ?", col.field_name.clone()));
                    }
                } else if col.col_type != Some("relation".to_owned()) {
                    if let Some(generater) = col.generator.clone() {
                        if generater.starts_with("mod_") {
                            // update_fields.push(format!("{} = ?", col.field_name.clone()));
                            update_fields.push(get_update_field_value_present(rb, &col.field_name.clone(), &col.field_type.clone().unwrap_or_default()));
                            answers.push(self.get_generated_value(&generater, jwt));
                        } else {
                            answers.push(rbs::to_value!(v.to_owned()));
                            // update_fields.push(format!("{} = ?", col.field_name.clone()));
                            update_fields.push(get_update_field_value_present(rb, &col.field_name.clone(), &col.field_type.clone().unwrap_or_default()));
                        }
                    } else {

                        if is_desensitize_with_crypto_store(&col.desensitize, col.crypto_store) {
                            let text = v.as_str().unwrap_or_default();
                            let crypto_text = crypto_desenstize_process(text.to_owned(), &ns, &col.desensitize);
                            answers.push(rbs::to_value!(crypto_text));
                        } else {
                            answers.push(rbs::to_value!(v.to_owned()));
                        }
                        // update_fields.push(format!("{} = ?", col.field_name.clone()));
                        update_fields.push(get_update_field_value_present(rb, &col.field_name.clone(), &col.field_type.clone().unwrap_or_default()));
                    }
                } else if col.col_type == Some("relation".to_owned()) && !col.relation_array {
                    let rel_field = col.relation_field.clone().unwrap_or_default();
                    // update_fields.push(format!("{} = ?", col.field_name.clone()));
                    update_fields.push(get_update_field_value_present(rb, &col.field_name.clone(), &col.field_type.clone().unwrap_or_default()));                    
                    if let Some(smv) = v.get(&rel_field) {
                        answers.push(rbs::to_value!(smv.to_owned()));
                    } else {
                        answers.push(rbs::to_value!(v.to_owned()));
                    }
                }
            } else if col.col_type != Some("relation".to_owned()) {
                if let Some(generater) = col.generator.clone() {
                    if generater.starts_with("mod_") {
                        update_fields.push(get_update_field_value_present(rb, &col.field_name.clone(), &col.field_type.clone().unwrap_or_default()));
                        // update_fields.push(format!("{} = ?", col.field_name.clone()));
                        answers.push(self.get_generated_value(&generater, jwt));
                    }
                }
            }
        }

        if with_key {
            answers.append(&mut answers_keys);
        }
        (answers, update_fields.join(","), key_fields.join(" and "))
    }

    fn to_select_sql(&self, with_key: bool, with_blob: bool, perm_sql: Option<String>) -> String {
        let sql = if self.0.select_sql.is_empty() {
            let mut text = String::from("select ");
            let mut field_str = vec![];

            for fl in self
                .0
                .fields
                .iter()
                .sorted_by(|a, b| Ord::cmp(&a.field_name, &b.field_name))
                .dedup_by(|x, y| x.field_name == y.field_name)
                .cloned()
            {
                if !fl.detail_only || with_blob {
                    field_str.push(format!("_tbl.{}", fl.field_name.clone()));
                }
            }

            text.push_str(field_str.join(",").as_str());

            let keycond = if with_key {
                self.0
                    .get_key_columns()
                    .into_iter()
                    .map(|f| format!(" {} = ? ", f.field_name))
                    .join(" AND ")
            } else {
                " 1 = 1 ".to_owned()
            };

            text.push_str(
                format!(
                    " from {} _tbl {} where {}",
                    self.0.object_name.clone(),
                    perm_sql.unwrap_or_default(),
                    keycond
                )
                .as_str(),
            );
            text
        } else {
            self.0.select_sql.clone()
        };
        sql
    }

    pub fn to_condition(&self, qs: &Value) -> Result<QueryCondition, Error> {
        serde_json::from_value::<QueryCondition>(qs.to_owned()).map_err(|err| anyhow!(err))
    }

    pub fn has_relationship(&self) -> bool {
        self.0
            .fields
            .clone()
            .into_iter()
            .any(|p| p.col_type == Some("relation".to_string()))
    }

    pub fn has_desensitize(&self) -> bool {
        self.0
            .fields
            .clone()
            .into_iter()
            .any(|p| p.desensitize.is_some() && p.desensitize != Some("none".to_owned()))
    }

    pub fn get_keys(&self) -> Vec<String> {
        self.0.get_key_columns().iter().map(|f| f.field_name.clone()).collect_vec()
    }

    pub fn generate_permission_sql(&self) -> Option<String> {
        if self.0.data_permission && self.2.data_permission {
            let permit_sql = format!(
                " INNER JOIN {} __p ON __p.{} = _tbl.{} AND __p.{} = ? ",
                self.2.relative_table.clone().unwrap_or_default(),
                self.0
                    .relative_field
                    .clone()
                    .unwrap_or(self.2.permit_relative_field.clone().unwrap_or_default()),
                self.0.permission_field.clone().unwrap_or_default(),
                self.2.permit_userfield.clone().unwrap_or_default()
            );
            Some(permit_sql)
        } else {
            None
        }
    }

    pub fn generate_permission_update_sql(&self) -> Option<String> {
        if self.0.data_permission && self.2.data_permission {
            let permit_sql = format!(
                " and {} IN (select __p.{} from {} __p where __p.{} = ?) ",
                self.0.permission_field.clone().unwrap_or_default(),
                self.0
                    .relative_field
                    .clone()
                    .unwrap_or(self.2.permit_relative_field.clone().unwrap_or_default()),
                self.2.relative_table.clone().unwrap_or_default(),
                self.2.permit_userfield.clone().unwrap_or_default()
            );
            Some(permit_sql)
        } else {
            None
        }
    }

    async fn delete_casc(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        ro: Value,
    ) -> Result<(), anyhow::Error> {
        for col in self
            .0
            .clone()
            .fields
            .iter()
            .filter(|f| f.col_type == Some("relation".to_string()))
        {
            if let Some(pkval) = ro.get(col.prop_name.clone().unwrap_or(col.field_name.clone())) {
                if let Some(relation_object) = col.relation_object.clone() {
                    if let Some(relation) = self.1.get_object(&relation_object) {
                        let dbx = DbStoreObject(relation.clone(), self.1.clone(), self.2.clone());
                        if col.relation_array {
                            let valkey = if pkval.is_array() || pkval.is_object() {
                                ro.get(col.field_name.clone())
                                    .map(|f| f.to_owned())
                                    .unwrap_or(Value::Null)
                            } else {
                                pkval.clone()
                            };
                            let ci = ConditionItem {
                                field: col.relation_field.clone().unwrap_or_default(),
                                op: "=".to_owned(),
                                value: valkey,
                                ..Default::default()
                            };
                            let qs = QueryCondition {
                                and: vec![ci],
                                ..Default::default()
                            };
                            let _ = dbx.delete_by_(executor.clone(), jwt, &qs).await?;
                        } else {
                            let pkey = dbx.get_primary_key_field().unwrap_or("id".to_owned());
                            if let Some(keyval) = pkval.get(&pkey) {
                                let ci = ConditionItem {
                                    field: pkey.clone(),
                                    op: "=".to_owned(),
                                    value: keyval.to_owned(),
                                    ..Default::default()
                                };
                                let qs = QueryCondition {
                                    and: vec![ci],
                                    ..Default::default()
                                };
                                let _ = dbx.delete_by_(executor.clone(), jwt, &qs).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn insert_(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &Value,
    ) -> Result<Value, Error> {
        let mt = if self.has_relationship() {
            match insert_casc_2(
                &self.0.clone(),
                &self.1.clone(),
                &self.2.clone(),
                executor.clone(),
                jwt,
                t,
                false,
            )
            .await
            {
                Ok(val) => copy_value_replaced(t, &val),
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            t.clone()
        };

        let (args, insert_fields, insert_values) = self.to_insert_rbs_value_vec(&executor.rb, jwt, &mt, &Value::Null);
        let sql = format!(
            "insert into {} ({}) values({})",
            self.0.object_name.clone(),
            insert_fields,
            insert_values
        );

        match executor.exec(&sql, args).await {
            Ok(ts) => {
                let mut smt = mt.clone();
                self.0
                    .fields
                    .clone()
                    .into_iter()
                    .filter(|p| p.pkey)
                    .for_each(|f| {
                        if let Some(mxt) = smt.as_object_mut() {
                            log::info!("show mxt: {:?}", mxt);
                            mxt.insert(
                                f.prop_name.unwrap_or(f.field_name),
                                json!(ts.last_insert_id),
                            );
                        }
                    });
                log::info!("show mxt inserted: {:?}", smt);
                if self.has_relationship() {
                    match insert_casc_2(
                        &self.0.clone(),
                        &self.1.clone(),
                        &self.2.clone(),
                        executor,
                        jwt,
                        t,
                        true,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                Ok(smt)
            }
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    async fn update_(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &Value,
    ) -> Result<Value, Error> {
        let perm_sql = self.generate_permission_update_sql();

        let mt = if self.has_relationship() {
            match insert_casc_2(
                &self.0.clone(),
                &self.1.clone(),
                &self.2.clone(),
                executor.clone(),
                jwt,
                t,
                false,
            )
            .await
            {
                Ok(val) => copy_value_replaced(t, &val),
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            t.clone()
        };

        let (mut args, update_fields, update_keys) = self.to_update_rbs_value_vec(&executor.rb, jwt,&mt, true);
        let sql = format!(
            "update {} set {} where {} {}",
            self.0.object_name.clone(),
            update_fields,
            update_keys,
            perm_sql.clone().unwrap_or_default()
        );

        if perm_sql.is_some() {
            args.push(rbs::to_value!(jwt.userid.clone()));
        }

        match executor.exec(&sql, args).await {
            Ok(_) => {
                if self.has_relationship() {
                    match insert_casc_2(
                        &self.0.clone(),
                        &self.1.clone(),
                        &self.2.clone(),
                        executor,
                        jwt,
                        t,
                        true,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                Ok(t.clone())
            }
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    /**
     * 根据条件来判断是执行Update操作或Insert操作
     */
    async fn upsert_(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: &Value,
        qs: Option<QueryCondition>,
    ) -> Result<Value, Error> {
        let qx = qs.unwrap_or_else(|| {
            let mut m = QueryCondition::default();
            for k in self.0.get_key_columns().iter().map(|f| f.field_name.clone()).collect_vec() {
                let ci = ConditionItem {
                    field: k.clone(),
                    op: "=".to_owned(),
                    value: self.get_field_value(&k, val),
                    ..Default::default()
                };
                m.and.push(ci);
            }
            m
        });

        // let perm_sql = self.generate_permission_sql();
        let mut sql = self.to_select_sql(false, false, None);
        let (cond_sql, cond_args) = qx.to_query(true)?;

        if qx.is_empty_condition() {
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(" AND ");
            sql.push_str(&cond_sql);
        }

        let args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        match executor.query_decode::<Vec<Value>>(&sql, args).await {
            Ok(vrs) => {
                if vrs.is_empty() {
                    // do insert.
                    self.insert_(executor, jwt, val).await
                } else if vrs.len() == 1 {
                    // do update
                    let new_val = copy_value_excluded(val, &self.0.get_key_columns().iter().map(|f| f.field_name.clone()).collect_vec());
                    let upd_val = copy_value_replaced(&vrs[0], &new_val);
                    self.update_(executor, jwt, &upd_val).await
                } else {
                    // report an error
                    Err(anyhow!("Upsert could not be executed when there are many records by this condition."))
                }
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    /**
     * Delete操作
     * 如果该StoreObject定义了级联删除，则会执行级联删除操作
     * 否则，它将只删除主表数据
     */
    async fn delete_by_(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Value, Error> {
        let perm_sql = self.generate_permission_update_sql();
        let (cond, cond_args) = qs.to_query(true)?;
        let mut sql = format!(
            "delete from {} where 1 = 1 ",
            self.0.object_name
        );

        if qs.is_empty_condition() {
            sql.push_str(" AND ");
            sql.push_str(&cond);
        }

        sql.push_str(&perm_sql.clone().unwrap_or_default());

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        if perm_sql.is_some() {
            args.push(rbs::to_value!(jwt.userid.clone()));
        }

        match executor.exec(&sql, args).await {
            Ok(rs) => Ok(json!({"rows_affected": rs.rows_affected})),
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }
}

impl DbCrud<Value> for DbStoreObject {
    async fn insert(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &Value,
    ) -> Result<Value, Error> {
        validate_object(t, &self.0, false)?;

        let mt = if self.has_relationship() {
            match insert_casc_1(
                &self.0.clone(),
                &self.1.clone(),
                &self.2.clone(),
                executor.clone(),
                jwt,
                t,
                false,
            )
            .await
            {
                Ok(val) => {
                    log::info!("Insert_casc_1: {:?}", val);
                    copy_value_replaced(t, &val)
                }
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            t.clone()
        };

        log::info!("Debug: {:?}", mt);

        let (args, insert_fields, insert_values) = self.to_insert_rbs_value_vec(&executor.rb, jwt,&mt, &Value::Null);
        let sql = format!(
            "insert into {} ({}) values({})",
            self.0.object_name.clone(),
            insert_fields,
            insert_values
        );

        match executor.exec(&sql, args).await {
            Ok(ts) => {
                let mut smt = mt.clone();
                self.0
                    .fields
                    .clone()
                    .into_iter()
                    .filter(|p| p.pkey)
                    .for_each(|f| {
                        if let Some(mxt) = smt.as_object_mut() {
                            mxt.insert(
                                f.prop_name.unwrap_or(f.field_name),
                                json!(ts.last_insert_id),
                            );
                        }
                    });
                if self.has_relationship() {
                    match insert_casc_1(
                        &self.0.clone(),
                        &self.1.clone(),
                        &self.2.clone(),
                        executor,
                        jwt,
                        &smt,
                        true,
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }
                Ok(smt)
            }
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    async fn update(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &Value,
    ) -> Result<Value, Error> {
        let perm_sql = self.generate_permission_update_sql();

        validate_object(t, &self.0, true)?;

        let mt = if self.has_relationship() {
            match insert_casc_1(
                &self.0.clone(),
                &self.1.clone(),
                &self.2.clone(),
                executor.clone(),
                jwt,
                t,
                false,
            )
            .await
            {
                Ok(val) => copy_value_replaced(t, &val),
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            t.clone()
        };

        let mt = if self.has_desensitize() {
            let keeps = self.get_keys();
            match self.select(Arc::new(executor.rb.clone()), jwt, &mt).await {
                Ok(v) => {
                    if let Some(v) = v {
                        // 比较mt中，与v中的每一个字段是否相同
                        // 相同则不作更新，脱敏字段，不作改变
                        copy_value_compared_replaced(&v, &mt, true, &keeps)
                    } else {
                        mt
                    }
                }
                Err(err) => {
                    log::warn!("could not select a value {err}");
                    mt
                }
            }
        } else {
            mt
        };

        let (mut args, update_fields, update_keys) = self.to_update_rbs_value_vec(&executor.rb, jwt,&mt, true);
        if update_fields.is_empty() {
            return Ok(mt);
        }
        let sql = format!(
            "update {} set {} where {} {}",
            self.0.object_name.clone(),
            update_fields,
            update_keys,
            perm_sql.clone().unwrap_or_default()
        );

        if perm_sql.is_some() {
            args.push(rbs::to_value!(jwt.userid.clone()));
        }

        match executor.exec(&sql, args).await {
            Ok(rr) => {
                if rr.rows_affected > 0 {
                    if self.has_relationship() {
                        match insert_casc_1(
                            &self.0.clone(),
                            &self.1.clone(),
                            &self.2.clone(),
                            executor,
                            jwt,
                            t,
                            true,
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                return Err(err);
                            }
                        }
                    }
                    Ok(t.clone())
                } else {
                    Err(anyhow!("Nothing to updated"))
                }
            }
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    /**
     * 根据条件来判断是执行Update操作或Insert操作
     */
    async fn upsert(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: &Value,
        qs: Option<QueryCondition>,
    ) -> Result<Value, Error> {
        validate_object(val, &self.0, false)?;
        let qx = qs.unwrap_or_else(|| {
            let mut m = QueryCondition::default();
            for k in self.0.get_key_columns().iter().map(|f| f.field_name.clone()).collect_vec() {
                let ci = ConditionItem {
                    field: k.clone(),
                    op: "=".to_owned(),
                    value: self.get_field_value(&k, val),
                    ..Default::default()
                };
                m.and.push(ci);
            }
            m
        });

        log::info!("QS: {:?}", qx);

        // let perm_sql = self.generate_permission_sql();
        let mut sql = self.to_select_sql(false, false, None);
        let (cond_sql, cond_args) = qx.to_query(true)?;
        if qx.is_empty_condition() {
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(" AND ");
            sql.push_str(&cond_sql);
        }


        let args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        let rs = match executor.query(&sql, args).await {
            Ok(rs) => rs,
            Err(err) => {
                log::info!("test error : {:?}", err);
                return Err(anyhow::Error::new(err));
            }
        };

        match decode_vec_custom_fields_list(
            executor.clone(),
            jwt,
            &self.1,
            rs,
            &self.0.fields,
            &self.1.namespace,
        )
        .await
        {
            Ok(vrs) => {
                if vrs.is_empty() {
                    // do insert.
                    self.insert(executor, jwt, val).await
                } else if vrs.len() == 1 {
                    // do update
                    let new_val = copy_value_excluded(val, &self.0.get_key_columns().iter().map(|f| f.field_name.clone()).collect_vec());
                    let upd_val = copy_value_replaced(&vrs[0], &new_val);
                    self.update(executor, jwt, &upd_val).await
                } else {
                    // report an error
                    Err(anyhow!("Upsert could not be executed when there are many records by this condition."))
                }
            }
            Err(err) => Err(anyhow!(err)),
        }
    }


    async fn save_batch(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: Vec<Value>
    ) -> Result<Value, Error> {
        let mut it: i64 = 0;
        for ic in val {
            let qst = if let Some(qsval) = ic.get("_cond") {
                if let Ok(qs) = serde_json::from_value::<QueryCondition>(qsval.to_owned()) {
                    Some(qs)
                } else {
                    None
                }
            } else {
                None
            };

            self.upsert(executor.clone(), jwt, &ic, qst).await?;
            it += 1;
        }

        Ok(json!({"affect_rows": it}))
    }

    async fn delete(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        t: &Value,
    ) -> Result<Value, Error> {
        validate_object(t, &self.0, true)?;

        let perm_sql = self.generate_permission_update_sql();
        let keycond = self
            .0
            .get_key_columns()
            .clone()
            .into_iter()
            .map(|f| format!(" {} = ? ", f.field_name))
            .join(" AND ");
        let del_sql = format!(
            "delete from {} where {} {}",
            self.0.object_name.clone(),
            keycond,
            perm_sql.clone().unwrap_or_default()
        );

        if self.has_relationship() {
            let exec: Arc<dyn Executor> = Arc::new(executor.rb.clone());
            if let Ok(Some(tv)) = self.select(exec, jwt, t).await {
                self.delete_casc(executor.clone(), jwt, tv).await?;
            }
        }

        let mut args = self.get_keys_values(t);

        if perm_sql.is_some() {
            args.push(rbs::to_value!(jwt.userid.clone())); // should guess the field type of userid
        }

        match executor.exec(&del_sql, args).await {
            Ok(_) => Ok(t.clone()),
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    async fn select(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        t: &Value,
    ) -> Result<Option<Value>, Error> {
        let perm_sql = self.generate_permission_sql();
        let mut args = self.get_keys_values(t);
        let sql = self.to_select_sql(true, true, perm_sql.clone());
        log::info!("Select Query: {}", sql.clone());

        if let Some(t) = perm_sql {
            if !t.is_empty() {
                args.insert(0, rbs::to_value!(jwt.userid.clone()));
            }
        }

        // let conn = rb.acquire().await?;
        let rs = match rb.query(&sql, args).await {
            Ok(rs) => rs,
            Err(err) => {
                log::info!("test error : {:?}", err);
                return Err(anyhow::Error::new(err));
            }
        };
        match decode_vec_custom_fields_list(
            rb,
            jwt,
            &self.1,
            rs,
            &self.0.fields,
            &self.1.namespace,
        )
        .await
        {
            Ok(mp) => {
                if mp.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(mp[0].to_owned()))
                }
            }
            Err(err) => Err(err),
        }
    }

    async fn find_one(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Option<Value>, Error> {
        let perm_sql = self.generate_permission_sql();
        let mut sql = self.to_select_sql(false, true, perm_sql.clone());
        let (cond_sql, cond_args) = qs.to_query(false)?;

        if qs.is_empty_condition() {
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(" AND ");
            sql.push_str(&cond_sql);
        }

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        if let Some(t) = perm_sql {
            if !t.is_empty() {
                args.insert(0, rbs::to_value!(jwt.userid.clone()));
            }
        }

        log::info!("FindOne Query: {}", sql.clone());
        match rb.query(&sql, args).await {
            Ok(rs) => {
                match decode_vec_custom_fields_list(
                    rb,
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields,
                    &self.1.namespace,
                )
                .await
                {
                    Ok(mp) => {
                        if mp.is_empty() {
                            Ok(None)
                        } else {
                            Ok(Some(mp[0].to_owned()))
                        }
                    }
                    Err(err) => Err(err),
                }
            }
            Err(err) => {
                log::info!("test error : {:?}", err);
                Err(anyhow::Error::new(err))
            }
        }
    }

    async fn query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Vec<Value>, Error> {
        let perm_sql = self.generate_permission_sql();
        let mut sql = self.to_select_sql(false, false, perm_sql.clone());
        let (cond_sql, cond_args) = qs.to_query(false)?;

        if qs.is_empty_condition() {
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(" AND ");
            sql.push_str(&cond_sql);
        }        

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        if let Some(t) = perm_sql {
            if !t.is_empty() {
                args.insert(0, rbs::to_value!(jwt.userid.clone()));
            }
        }

        log::info!("Query: {}", sql.clone());
        match rb.query(&sql, args).await {
            Ok(rs) => {
                match decode_vec_custom_fields_list(
                    rb,
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields,
                    &self.1.namespace,
                )
                .await
                {
                    Ok(mp) => Ok(mp),
                    Err(err) => Err(err),
                }
            }
            Err(err) => {
                log::info!("test error : {:?}", err);
                Err(anyhow::Error::new(err))
            }
        }
    }

    async fn paged_query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Page<Value>, Error> {
        let perm_sql = self.generate_permission_sql();
        let mut sql = self.to_select_sql(false, false, perm_sql.clone());
        let pagereq = match qs.to_page_request() {
            Some(p) => p,
            None => {
                return Err(anyhow::anyhow!("No Paging suppliered."));
            }
        };

        let mut count_sql = format!(
            "select count(1) from {} where 1 = 1 ",
            self.0.object_name.clone()
        );

        let (cond_sql, cond_args) = qs.to_query(false)?;


        if qs.is_empty_condition() {
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(" AND ");
            sql.push_str(&cond_sql);
        }

        sql.push_str(
            format!(
                " limit {} offset {} ",
                pagereq.page_size(),
                pagereq.offset()
            )
            .as_str(),
        );

        if qs.is_empty_condition() {
            count_sql.push_str(&cond_sql);
        } else {
            count_sql.push_str(" AND ");
            count_sql.push_str(&cond_sql);
        }

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        if let Some(t) = perm_sql {
            if !t.is_empty() {
                args.insert(0, rbs::to_value!(jwt.userid.clone()));
            }
        }

        log::info!("Query: {}", sql.clone());

        let total = match rb.query(&count_sql, args.clone()).await {
            Ok(rs) => {
                match rbatis::decode::<Value>(rs) {
                    Ok(rts) => {
                        match rts {
                            Value::Array(tm) => {
                                match tm.first().map(|f| f.to_owned()).unwrap_or(Value::Null) {
                                    Value::Object(tx) => {
                                        tx.into_iter().map(|(_k, v)| v.as_u64().unwrap_or(0u64)).last().unwrap_or(0u64)
                                    },
                                    _ => {
                                        if tm.is_empty() {
                                            0u64
                                        } else {
                                            tm[0].as_u64().unwrap_or(0u64)   
                                        }
                                    }
                                }
                            },
                            _ => rts.as_u64().unwrap_or(0u64)
                        }
                        
                    },
                    Err(e) => {
                        log::info!("Error {e}");
                        0u64
                    }
                }
            },
            Err(err) => {
                log::info!("Error : {}", err);
                0u64
            }
        };
        
        match rb.query(&sql, args).await {
            Ok(rs) => {
                match decode_vec_custom_fields_list(
                    rb,
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields,
                    &self.1.namespace,
                )
                .await
                {
                    Ok(mp) => Ok(
                        Page::new_total(pagereq.page_no(), pagereq.page_size(), total)
                            .set_records(mp),
                    ),
                    Err(err) => Err(err),
                }
            }
            Err(err) => {
                log::info!("test error : {:?}", err);
                Err(anyhow::Error::new(err))
            }
        }
    }

    /**
     * Delete操作
     * 如果该StoreObject定义了级联删除，则会执行级联删除操作
     * 否则，它将只删除主表数据
     */
    async fn delete_by(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        qs: &QueryCondition,
    ) -> Result<Value, Error> {
        let perm_sql = self.generate_permission_update_sql();
        let (cond, cond_args) = qs.to_query(true)?;
        let mut sql = format!(
            "delete from {} where 1 = 1 ",
            self.0.object_name
        );

        if !qs.is_empty_condition() {
            sql.push_str(" AND ");
            sql.push_str(&cond);
        }

        sql.push_str(&perm_sql.clone().unwrap_or_default());

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        if perm_sql.is_some() {
            args.push(rbs::to_value!(jwt.userid.clone()));
        }

        if self.has_relationship() {
            let rb = executor.rb.clone();
            let arcxe: Arc<dyn Executor> = Arc::new(rb);
            match self.query(arcxe, jwt, qs).await {
                Ok(rs) => {
                    for ro in rs {
                        self.delete_casc(executor.clone(), jwt, ro).await?
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            };
        }

        match executor.exec(&sql, args).await {
            Ok(rs) => Ok(json!({"rows_affected": rs.rows_affected})),
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    /**
     * update_by操作不支持包含Relation的附属表的更新操作
     */
    async fn update_by(
        &self,
        executor: Arc<RBatisTxExecutor>,
        jwt: &JwtUserClaims,
        val: &Value,
        qs: &QueryCondition,
    ) -> Result<Value, Error> {
        validate_object(val, &self.0, false)?;

        let perm_sql = self.generate_permission_update_sql();

        if qs.is_empty_condition() {
            return Err(anyhow!("No condition provided."));
        }

        let (cond, cond_args) = qs.to_query(true)?;


        let (mut args, update_fields, _) = self.to_update_rbs_value_vec(&executor.rb, jwt, val, false);
        let sql = format!(
            "update {} set {} where {} {}",
            self.0.object_name.clone(),
            update_fields,
            cond,
            perm_sql.clone().unwrap_or_default()
        );

        let mut c_args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        args.append(&mut c_args);

        if perm_sql.is_some() {
            args.push(rbs::to_value!(jwt.userid.clone()));
        }

        match executor.exec(&sql, args).await {
            Ok(rs) => Ok(json!({"rows_affected": rs.rows_affected})),
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }
}
