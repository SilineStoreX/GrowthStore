use std::{collections::HashMap, sync::{Arc, Mutex}};

use anyhow::{anyhow, Error};

use chimes_store_core::{
    config::{
        auth::{AuthorizationConfig, JwtUserClaims},
        Column, QueryCondition, QueryObject, StoreServiceConfig,
    },
    service::convert::create_convert_holder,
};
use itertools::Itertools;
use rbatis::{executor::Executor, IPageRequest, Page};
use regex::Regex;
use serde_json::{json, Value};
use substring::Substring;

use crate::dbs::{crud::validate_query_object, decode_vec_custom_fields};

pub struct DbQueryObject(
    pub QueryObject,
    pub StoreServiceConfig,
    pub AuthorizationConfig,
);

unsafe impl Send for DbQueryObject {}

unsafe impl Sync for DbQueryObject {}

impl DbQueryObject {
    pub fn make_fixed_params_args(
        &self,
        sql: &str,
        jwt: &JwtUserClaims,
        fix_param: &Value,
    ) -> (String, Vec<rbs::Value>) {
        let mut mut_sql = sql.to_string();
        let mut args = vec![];
        let mut query_params = self
            .0
            .params
            .clone()
            .into_iter()
            .filter(|p| p.pkey)
            .collect_vec();
        query_params.push(Column {
            pkey: true,
            field_name: "jwt.username".to_owned(),
            ..Default::default()
        });

        let sort_params = match Regex::new(r#"\#\{[\w\.]*\}"#) {
            Ok(regex) => {
                let mut resort_params = vec![];
                for mt in regex.find_iter(sql) {
                    let kp = mt.as_str();
                    // log::warn!("Keys: {kp}");
                    let key = if kp.starts_with("#{") && kp.ends_with("}") {
                        kp.substring(2, kp.len() - 1)
                    } else {
                        kp
                    };
                    query_params.iter().filter(|p| p.field_name == key.to_string()).for_each(|c| {
                        resort_params.push(c.to_owned());
                    });
                }
                // log::info!("Sorted: {resort_params:?}");
                resort_params
            },
            Err(err) => {
                log::warn!("Error of regex: {err}");
                query_params
            }
        };
        
        for col in sort_params {
            let prop_name = col.prop_name.unwrap_or(col.field_name.clone());
            let field_name = col.field_name.clone();
            let patt = format!("#{{{field_name}}}");

            if mut_sql.contains(&patt) {
                if field_name == "jwt.username" {
                    args.push(rbs::value!(jwt.username.clone()));
                } else {
                    args.push(rbs::value!(fix_param.get(&prop_name)));
                }
            }
            mut_sql = mut_sql.replace(&patt, "?");
        }

        (mut_sql, args)
    }

    #[allow(dead_code)]
    fn rewrite_count_sql(&self, sql: &str) -> String {
        let mut mut_sql = sql.to_string();
        let mut query_params = self
            .0
            .params
            .clone()
            .into_iter()
            .filter(|p| p.pkey)
            .collect_vec();
        query_params.push(Column {
            pkey: true,
            field_name: "jwt.username".to_owned(),
            ..Default::default()
        });

        for col in query_params {
            let field_name = col.field_name;
            let patt = format!("#{{{field_name}}}");
            mut_sql = mut_sql.replace(&patt, "?");
        }

        mut_sql
    }

    pub fn to_condition(&self, qs: &[Value]) -> Result<QueryCondition, Error> {
        if qs.len() > 1 {
            serde_json::from_value::<QueryCondition>(qs[1].to_owned()).map_err(|err| anyhow!(err))
        } else if qs.len() == 1 {
            let qs_args = qs[0].to_owned();
            if let Some(condqs) = qs_args.get("_cond") {
                serde_json::from_value::<QueryCondition>(condqs.to_owned()).map_err(|err| anyhow!(err))
            } else {
                Ok(QueryCondition::default())
            }
        } else {
            Ok(QueryCondition::default())
        }
    }

    pub fn generate_permission_sql(&self) -> Option<String> {
        if self.0.data_permission && self.2.data_permission {
            let permit_sql = format!(
                " INNER JOIN {} __p ON __p.{} = {} AND __p.{} = #{{jwt.username}} ",
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

    pub async fn direct_query(
        &self,
        rb: Arc<dyn Executor>,
        sql: &str,
        params: &[Value],
    ) -> Result<Vec<Value>, Error> {
        let (rwsql, args) = if params.len() == 1 {
            let mut rewrite_sql = sql.to_string(); // add direct query by Value::Object style by property_name
            if let Value::Object(mp) = params[0].clone() {
                let sort_params = match Regex::new(r#"\#\{[\w\.]*\}"#) {
                    Ok(regex) => {
                        let mut resort_params = vec![];
                        for mt in regex.find_iter(sql) {
                            let kp = mt.as_str();
                            let key = if kp.starts_with("#{") && kp.ends_with("}") {
                                kp.substring(2, kp.len() - 1)
                            } else {
                                kp
                            };
                            if let Some(val) = mp.get(key) {
                                resort_params.push(val.to_owned());
                            } else {
                                resort_params.push(Value::Null);
                            }
                            rewrite_sql = rewrite_sql.replace(kp, "?");
                        }
                        
                        resort_params
                    },
                    Err(err) => {
                        log::warn!("Error of regex: {err}");
                        params.to_vec()
                    }
                };

                (rewrite_sql, sort_params.iter().map(|v| rbs::value!(v)).collect_vec())
            } else {
                (rewrite_sql, params.iter().map(|v| rbs::value!(v)).collect_vec())
            }
        } else {
            (sql.to_string(), params.iter().map(|v| rbs::value!(v)).collect_vec())
        };


        match rb.query(&rwsql, args).await {
            Ok(rs) => match rbatis::decode::<Vec<Value>>(rs) {
                Ok(rets) => Ok(rets),
                Err(err) => Err(anyhow::Error::new(err)),
            },
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    pub async fn query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        fix_param: &Value,
        qs: &QueryCondition,
    ) -> Result<Vec<Value>, Error> {
        let driver_type = rb.driver_type().unwrap_or("mysql");
        let mut sql = self.0.query_body.clone();
        let (cond_sql, cond_args) = qs.to_query(&self.0, driver_type, false)?;

        // log::info!("original sql: {sql}");
        if !qs.is_empty_condition() {
            sql.push_str(" and ");
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(&cond_sql);
        }

        log::debug!("cond sql: {cond_sql}");

        // replace the ${DATA_PERMISSION_SQL}
        if jwt.superadmin {
            // remove ${DATA_PERMISSION_SQL} placeholder
            sql = sql.replace("${DATA_PERMISSION_SQL}", " ");
        } else if let Some(psql) = self.generate_permission_sql() {
            sql = sql.replace("${DATA_PERMISSION_SQL}", &psql);
        }

        let (rw_sql, mut fixed_args) = self.make_fixed_params_args(&sql, jwt, fix_param);

        let mut args = cond_args.into_iter().map(|v| rbs::value!(v)).collect_vec();

        fixed_args.append(&mut args);

        log::debug!("Query: {rw_sql}");
        match rb.query(&rw_sql, fixed_args).await {
            Ok(rs) => {
                let mut holder = create_convert_holder(&self.0.fields, &self.1).await;
                match decode_vec_custom_fields(
                    rb.clone(),
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields_map(),
                    &self.1.namespace,
                    &mut holder,
                    Arc::new(Mutex::new(HashMap::new()))
                )
                .await
                {
                    Ok(mp) => Ok(mp),
                    Err(err) => Err(err),
                }
            }
            Err(err) => {
                log::info!("test error : {err:?}");
                Err(anyhow::Error::new(err))
            }
        }
    }

    pub async fn execute(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        fix_param: &Value,
        qs: &QueryCondition,
    ) -> Result<Value, Error> {
        let driver_type = rb.driver_type().unwrap_or("mysql");
        let mut sql = self.0.query_body.clone();
        let (cond_sql, cond_args) = qs.to_query(&self.0, driver_type, false)?;

        if !qs.is_empty_condition() {
            sql.push_str(" and ");
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(&cond_sql);
        }

        // Only the execute (for update) will validate the params
        validate_query_object(&self.0.name, fix_param, &self.0, &self.1)?;

        // replace the ${DATA_PERMISSION_SQL}
        if jwt.superadmin {
            // remove ${DATA_PERMISSION_SQL} placeholder
            sql = sql.replace("${DATA_PERMISSION_SQL}", " ");
        } else if let Some(psql) = self.generate_permission_sql() {
            sql = sql.replace("${DATA_PERMISSION_SQL}", &psql);
        }

        let (rw_sql, mut fixed_args) = self.make_fixed_params_args(&sql, jwt, fix_param);

        let mut args = cond_args.into_iter().map(|v| rbs::value!(v)).collect_vec();

        fixed_args.append(&mut args);
        match rb.exec(&rw_sql, fixed_args).await {
            Ok(rs) => Ok(json!(rs)),
            Err(err) => {
                log::info!("test error : {err:?}");
                Err(anyhow::Error::new(err))
            }
        }
    }

    pub async fn paged_query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        fix_param: &Value,
        qs: &QueryCondition,
    ) -> Result<Page<Value>, Error> {
        let mut sql = self.0.query_body.clone();

        let pagereq = match qs.to_page_request() {
            Some(p) => p,
            None => {
                return Err(anyhow::anyhow!("No Paging suppliered."));
            }
        };

        // apply data permission
        if jwt.superadmin {
            // remove ${DATA_PERMISSION_SQL} placeholder
            sql = sql.replace("${DATA_PERMISSION_SQL}", " ");
        } else if let Some(psql) = self.generate_permission_sql() {
            sql = sql.replace("${DATA_PERMISSION_SQL}", &psql);
        }

        let mut count_sql =
            if self.0.count_query.is_none() || self.0.count_query == Some(String::new()) {
                format!("select count(1) from ({}", sql.clone())
            } else {
                let mut csql = self.0.count_query.clone().unwrap_or_default();
                if jwt.superadmin {
                    csql = csql.replace("${DATA_PERMISSION_SQL}", " ");
                } else if let Some(psql) = self.generate_permission_sql() {
                    csql = csql.replace("${DATA_PERMISSION_SQL}", &psql);
                }
                // self.rewrite_count_sql(&csql)
                csql
            };

        let driver_type = rb.driver_type().unwrap_or("mysql");
        let (cond_sql, cond_args) = qs.to_query(&self.0, driver_type, false)?;
        let (count_cond_sql, _cond_args) = qs.to_query(&self.0, driver_type, true)?;

        if !qs.is_empty_condition() {
            sql.push_str(" and ");
            sql.push_str(&cond_sql.clone());
            count_sql.push_str(" and ");
            count_sql.push_str(&count_cond_sql);
        } else {
            sql.push_str(&cond_sql.clone());
            count_sql.push_str(&count_cond_sql);
        }

        sql.push_str(
            format!(
                " limit {} offset {} ",
                pagereq.page_size(),
                pagereq.offset()
            )
            .as_str(),
        );

        let (rw_sql, mut fixed_args) = self.make_fixed_params_args(&sql, jwt, fix_param);

        let mut args = cond_args.into_iter().map(|v| rbs::value!(v)).collect_vec();

        // log::info!("args: {args:?}");
        // log::info!("fixed_args: {fixed_args:?}");
        fixed_args.append(&mut args.clone());

        // log::info!("Query: {}", rw_sql.clone());

        if self.0.count_query.is_none() || self.0.count_query == Some(String::new()) {
            count_sql.push_str(") a__");
        }

        // log::info!("CountSQL: {}", count_sql);

        let (rw_count_sql, mut fixed_count_args) =
            self.make_fixed_params_args(&count_sql, jwt, fix_param);

        // log::info!("fixed_count_args: {fixed_count_args:?}");
        // log::info!("args: {args:?}");
        fixed_count_args.append(&mut args);

        // log::info!("count args: {fixed_count_args:?}");
        // log::info!("base args: {fixed_args:?}");

        let total = match rb.query(&rw_count_sql, fixed_count_args.clone()).await {
            Ok(rs) => match rbatis::decode::<Value>(rs) {
                Ok(rts) => match rts {
                    Value::Array(tm) => {
                        match tm.first().map(|f| f.to_owned()).unwrap_or(Value::Null) {
                            Value::Object(tx) => tx
                                .into_iter()
                                .map(|(_k, v)| v.as_u64().unwrap_or(0u64))
                                .next_back()
                                .unwrap_or(0u64),
                            _ => {
                                if tm.is_empty() {
                                    0u64
                                } else {
                                    tm[0].as_u64().unwrap_or(0u64)
                                }
                            }
                        }
                    }
                    _ => rts.as_u64().unwrap_or(0u64),
                },
                Err(e) => {
                    log::info!("Error {e}");
                    0u64
                }
            },
            Err(err) => {
                log::info!("Error : {err}");
                0u64
            }
        };

        match rb.query(&rw_sql, fixed_args).await {
            Ok(rs) => {
                let mut holder = create_convert_holder(&self.0.fields, &self.1).await;
                match decode_vec_custom_fields(
                    rb.clone(),
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields_map(),
                    &self.1.namespace,
                    &mut holder,
                    Arc::new(Mutex::new(HashMap::new()))
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
                log::info!("test error : {err:?}");
                Err(anyhow::Error::new(err))
            }
        }
    }
}
