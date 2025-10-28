// DbConvert
// 使用字段映射方案来进行数据结构的改写。
// 支持Rhai脚本自定义的转换逻辑来改特殊的数据。
// 数据转换的作用为是为了对原始数据进行清洗。 所谓MapReduce中的Map过程

use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::{
    config::{Column, ConditionItem, OrdianlItem, QueryCondition, StoreServiceConfig},
    service::{invoker::InvocationContext, starter::MxStoreService},
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::sdk::InvokeUri;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RangeItemValue {
    pub dict_code: Option<String>,
    pub dict_sort: Option<i64>,
    pub greate_value: Option<String>,
    pub id: Option<i64>,
    pub item_label: Option<String>,
    pub less_value: Option<String>,
}

unsafe impl Send for RangeItemValue {}
unsafe impl Sync for RangeItemValue {}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DictItemValue {
    pub id: Option<i64>,
    pub dict_id: Option<i64>,
    pub dict_code: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub dict_sort: Option<i64>,
}

unsafe impl Send for DictItemValue {}
unsafe impl Sync for DictItemValue {}

#[derive(Debug, Clone)]
pub struct ConvertHolder {
    pub dict_uri: Option<String>,
    pub range_uri: Option<String>,
    pub dict_map: HashMap<String, Vec<DictItemValue>>,
    pub range_map: HashMap<String, Vec<RangeItemValue>>,
    pub subquery_map: HashMap<String, Option<Value>>,
    pub cached_map: HashMap<String, Option<Value>>,
}

impl Default for ConvertHolder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConvertHolder {
    pub fn new() -> Self {
        Self {
            dict_uri: None,
            range_uri: None,
            dict_map: HashMap::new(),
            range_map: HashMap::new(),
            subquery_map: HashMap::new(),
            cached_map: HashMap::new(),
        }
    }

    pub fn update_dict_uri(&mut self, dicturi: &Option<String>) {
        self.dict_uri = dicturi.clone();
    }

    pub fn update_range_uri(&mut self, rangeuri: &Option<String>) {
        self.range_uri = rangeuri.clone();
    }

    pub async fn transalte_dict(
        &mut self,
        name: &str,
        val: &Value,
    ) -> Result<Option<DictItemValue>, anyhow::Error> {
        if !self.dict_map.contains_key(name) {
            self.preload_dict(&[name.to_owned()]).await?;
        }

        let text = Some(match val {
            Value::String(t) => t.clone(),
            _ => val.to_string(),
        });

        log::warn!("text is {text:?}, {val:?}");

        if let Some(items) = self.dict_map.get(name) {
            for tp in items {
                log::warn!("cmp {:?} to {:?} is {}", tp.value, text, tp.value == text);
                if tp.value == text {
                    return Ok(Some(tp.to_owned()));
                }
            }
        }

        Ok(None)
    }

    pub async fn preload_dict(&mut self, dict_name: &[String]) -> Result<(), anyhow::Error> {
        if let Some(dicturi) = self.dict_uri.clone() {
            let ctx = Arc::new(Mutex::new(InvocationContext::new()));
            let qc = QueryCondition {
                and: vec![ConditionItem {
                    op: "IN".to_owned(),
                    field: "dict_code".to_owned(),
                    value: json!(dict_name),
                    value2: Value::Null,
                    ..Default::default()
                }],
                sorts: vec![
                    OrdianlItem {
                        field: "dict_code".to_owned(),
                        sort_asc: true,
                    },
                    OrdianlItem {
                        field: "dict_sort".to_owned(),
                        sort_asc: true,
                    },
                ],
                ..Default::default()
            };

            let list = MxStoreService::invoke_return_vec(dicturi, ctx, vec![json!(qc)]).await?;
            for it in list
                .iter()
                .map(|f| serde_json::from_value::<DictItemValue>(f.to_owned()).unwrap_or_default())
                .filter(|p| p.dict_code.is_some())
            {
                match self
                    .dict_map
                    .entry(it.dict_code.clone().unwrap_or_default())
                {
                    Entry::Occupied(mut oe) => {
                        oe.get_mut().push(it);
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(vec![it]);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn preload_ranges(&mut self, range_name: &[String]) -> Result<(), anyhow::Error> {
        if let Some(dicturi) = self.range_uri.clone() {
            let ctx = Arc::new(Mutex::new(InvocationContext::new()));
            let qc = QueryCondition {
                and: vec![ConditionItem {
                    op: "IN".to_owned(),
                    field: "dict_code".to_owned(),
                    value: json!(range_name),
                    value2: Value::Null,
                    ..Default::default()
                }],
                sorts: vec![
                    OrdianlItem {
                        field: "dict_code".to_owned(),
                        sort_asc: true,
                    },
                    OrdianlItem {
                        field: "dict_sort".to_owned(),
                        sort_asc: true,
                    },
                ],
                ..Default::default()
            };

            let list = MxStoreService::invoke_return_vec(dicturi, ctx, vec![json!(qc)]).await?;
            for it in list
                .iter()
                .map(|f| serde_json::from_value::<RangeItemValue>(f.to_owned()).unwrap_or_default())
                .filter(|p| p.dict_code.is_some())
            {
                match self
                    .range_map
                    .entry(it.dict_code.clone().unwrap_or_default())
                {
                    Entry::Occupied(mut oe) => {
                        oe.get_mut().push(it);
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(vec![it]);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn transalte_range<T: PartialOrd + Clone + FromStr + Debug>(
        &mut self,
        name: &str,
        val: T,
    ) -> Result<Option<RangeItemValue>, anyhow::Error> {
        if !self.range_map.contains_key(name) {
            self.preload_ranges(&[name.to_owned()]).await?;
        }

        if let Some(items) = self.range_map.get(name) {
            for tp in items {
                let less = tp.less_value.clone().and_then(|f| f.parse::<T>().ok());

                let great = tp.greate_value.clone().and_then(|f| f.parse::<T>().ok());

                // log::error!("Compare Val: {val:?}, {less:?}, {great:?}");

                if less.is_none() && great.is_none() {
                    return Ok(None);
                }

                if less.is_none() && val >= great.clone().unwrap() {
                    return Ok(Some(tp.to_owned()));
                }

                if great.is_none() && val <= less.clone().unwrap() {
                    return Ok(Some(tp.to_owned()));
                }

                if less.is_some()
                    && great.is_some()
                    && val <= less.unwrap()
                    && val >= great.unwrap()
                {
                    return Ok(Some(tp.to_owned()));
                }
            }
        }

        Ok(None)
    }

    /**
     * Support query_val only String and Number
     */
    pub async fn load_subquery(
        &mut self,
        query_uri: &str,
        query_val: &Value,
    ) -> Result<Option<Value>, anyhow::Error> {
        let query_def = query_uri.split(";").collect_vec();
        if query_def.len() >= 2 {
            let calluri = query_def[0].to_string();
            let paramname = query_def[1].to_string();
            let displaylabel = if query_def.len() == 2 {
                query_def[1].to_string()
            } else {
                query_def[2].to_string()
            };

            let uri = match InvokeUri::parse(&calluri) {
                Ok(u) => u,
                Err(_err) => return Ok(None),
            };

            // call select method, only need two param

            let fmtkey = format!("{calluri}-{query_val}");

            match self.subquery_map.entry(fmtkey) {
                Entry::Occupied(oe) => {
                    let value = oe.get();
                    Ok(value.to_owned())
                }
                Entry::Vacant(ve) => {
                    let cond = if uri.schema == *"object" && uri.method == *"select" {
                        query_val.clone()
                    } else {
                        let qc = QueryCondition {
                            and: vec![ConditionItem {
                                field: paramname.clone(),
                                op: "=".to_owned(),
                                value: query_val.clone(),
                                value2: Value::Null,
                                ..Default::default()
                            }],
                            ..Default::default()
                        };

                        json!(qc)
                    };

                    let ctx = Arc::new(Mutex::new(InvocationContext::new()));
                    if let Some(res) =
                        MxStoreService::invoke_return_one(calluri, ctx, vec![cond]).await?
                    {
                        let xret = match res {
                            Value::Object(tmap) => tmap.get(&displaylabel).map(|v| v.to_owned()),
                            _ => Some(res),
                        };
                        ve.insert(xret.clone());
                        Ok(xret)
                    } else {
                        ve.insert(None);
                        Ok(None)
                    }
                }
            }
        } else {
            Ok(None)
        }
    }
}

pub async fn create_convert_holder(fields: &[Column], conf: &StoreServiceConfig) -> ConvertHolder {
    let mut holder = ConvertHolder::new();
    // holder
    // preload the dicts

    holder.update_dict_uri(&conf.dict_uri);
    holder.update_range_uri(&conf.range_uri);

    let mut dicts = fields
        .iter()
        .filter(|p| p.desensitize == Some("dict".to_owned()))
        .filter_map(|f| f.conv_params.clone())
        .collect_vec();
    if !dicts.is_empty() {
        dicts.sort();
        dicts.dedup();
        if let Err(err) = holder.preload_dict(&dicts).await {
            log::info!("could not load the dicts {err}");
        }
    }

    let mut ranges = fields
        .iter()
        .filter(|p| p.desensitize == Some("range".to_owned()))
        .filter_map(|f| f.conv_params.clone())
        .collect_vec();
    if !ranges.is_empty() {
        ranges.sort();
        ranges.dedup();
        if let Err(err) = holder.preload_ranges(&ranges).await {
            log::info!("could not load the ranges {err}");
        }
    }

    holder
}
