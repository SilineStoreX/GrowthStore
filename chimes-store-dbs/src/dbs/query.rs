use std::sync::Arc;

use anyhow::{anyhow, Error};

use chimes_store_core::config::{
    auth::{AuthorizationConfig, JwtUserClaims},
    Column, QueryCondition, QueryObject, StoreServiceConfig,
};
use itertools::Itertools;
use rbatis::{executor::Executor, IPage, IPageRequest, Page};
use serde_json::Value;

use crate::dbs::decode_vec_custom_fields;

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
        for col in query_params {
            let prop_name = col.prop_name.unwrap_or(col.field_name.clone());
            let field_name = col.field_name.clone();
            let patt = format!("#{{{}}}", field_name);

            if mut_sql.contains(&patt) {
                if field_name == "jwt.username" {
                    args.push(rbs::to_value!(jwt.username.clone()));
                } else {
                    args.push(rbs::to_value!(fix_param.get(&prop_name)));
                }
            }
            mut_sql = mut_sql.replace(&patt, "?");
        }

        (mut_sql, args)
    }

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
            let patt = format!("#{{{}}}", field_name);
            mut_sql = mut_sql.replace(&patt, "?");
        }

        mut_sql
    }

    pub fn to_condition(&self, qs: &[Value]) -> Result<QueryCondition, Error> {
        if qs.len() > 1 {
            serde_json::from_value::<QueryCondition>(qs[1].to_owned()).map_err(|err| anyhow!(err))
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
        params: &[Value]
    ) -> Result<Vec<Value>, Error> {
        let args = params
            .iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        match rb.query(sql, args).await {
            Ok(rs) => {
                match rbatis::decode::<Vec<Value>>(rs) {
                    Ok(rets) => {
                        Ok(rets)
                    },
                    Err(err) => {
                        Err(anyhow::Error::new(err))
                    },
                }
            }
            Err(err) => {
                Err(anyhow::Error::new(err))
            }
        }
    }

    pub async fn query(
        &self,
        rb: Arc<dyn Executor>,
        jwt: &JwtUserClaims,
        fix_param: &Value,
        qs: &QueryCondition,
    ) -> Result<Vec<Value>, Error> {
        let mut sql = self.0.query_body.clone();
        let (cond_sql, cond_args) = qs.to_query(false)?;

        if !qs.is_empty_condition() {
            sql.push_str(" and ");
            sql.push_str(&cond_sql);
        } else {
            sql.push_str(&cond_sql);
        }

        // replace the ${DATA_PERMISSION_SQL}
        if jwt.superadmin {
            // remove ${DATA_PERMISSION_SQL} placeholder
            sql = sql.replace("${DATA_PERMISSION_SQL}", " ");
        } else if let Some(psql) = self.generate_permission_sql() {
            sql = sql.replace("${DATA_PERMISSION_SQL}", &psql);
        }

        let (rw_sql, mut fixed_args) = self.make_fixed_params_args(&sql, jwt, fix_param);

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        fixed_args.append(&mut args);
        match rb.query(&rw_sql, fixed_args).await {
            Ok(rs) => {
                match decode_vec_custom_fields(
                    rb.clone(),
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields_map(),
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
                self.rewrite_count_sql(&csql)
            };

        let (cond_sql, cond_args) = qs.to_query(false)?;

        if !qs.is_empty_condition() {
            sql.push_str(" and ");
            sql.push_str(&cond_sql.clone());
            count_sql.push_str(" and ");
            count_sql.push_str(&cond_sql);
        } else {
            sql.push_str(&cond_sql.clone());
            count_sql.push_str(&cond_sql);
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

        let mut args = cond_args
            .into_iter()
            .map(|v| rbs::to_value!(v))
            .collect_vec();

        fixed_args.append(&mut args.clone());

        log::info!("Query: {}", rw_sql.clone());

        if self.0.count_query.is_none() || self.0.count_query == Some(String::new()) {
            count_sql.push_str(") a__");
        }

        log::info!("CountSQL: {}", count_sql);

        let (rw_count_sql, mut fixed_count_args) =
            self.make_fixed_params_args(&count_sql, jwt, fix_param);

        fixed_count_args.append(&mut args);


        let total = match rb.query(&rw_count_sql, fixed_count_args.clone()).await {
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

        match rb.query(&rw_sql, fixed_args).await {
            Ok(rs) => {
                match decode_vec_custom_fields(
                    rb.clone(),
                    jwt,
                    &self.1,
                    rs,
                    &self.0.fields_map(),
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
}
