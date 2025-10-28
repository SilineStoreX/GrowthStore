use crate::proc::template::json_path_get_string;
use crate::proc::template::template_eval;
use anyhow::anyhow;
use chimes_store_core::config::PluginConfig;
use chimes_store_core::service::invoker::InvocationContext;
use chimes_store_core::service::sdk::{InvokeUri, MethodDescription, RxPluginService};
use chimes_store_core::service::starter::load_config;
use chimes_store_utils::common::option_value_to_vec_value;
use chimes_store_utils::template::json_path_get;
use elasticsearch::auth::Credentials;
use elasticsearch::http::transport::{SingleNodeConnectionPool, Transport, TransportBuilder};
use elasticsearch::http::StatusCode;
use elasticsearch::{CreateParts, DeleteParts, Elasticsearch, GetParts, SearchParts, UpdateParts};
use rbatis::Page;
use reqwest::Url;
use salvo::oapi::Array;
use salvo::oapi::{
    BasicType, Content, Object, OpenApi, Operation, PathItem, RefOr, RequestBody, Response, Schema,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

fn to_api_result_schema(t: RefOr<Schema>, array: bool) -> Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property("status", Object::new().schema_type(BasicType::Integer));
    apiresult = apiresult.property("message", Object::new().schema_type(BasicType::String));
    if array {
        apiresult = apiresult.property("data", Array::new().items(t));
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property("timestamp", Object::new().schema_type(BasicType::Integer));
    Schema::Object(Box::new(apiresult))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ElasticSearchTargetInfo {
    pub name: String,
    pub op_type: Option<String>,
    pub index: Option<String>,
    pub id_field: Option<String>,
    pub api_type: Option<String>,
    pub description: Option<String>,
    pub request_method: Option<String>,
    pub request_body: Option<String>,
    pub lang: Option<String>,
    pub script: Option<String>,
    pub return_validate: Option<String>,
    pub return_data: Option<String>,
}

impl ElasticSearchTargetInfo {
    pub(crate) fn to_operation(&self, array: bool) -> Operation {
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(RequestBody::new().add_content(
            "application/json",
            RefOr::Type(Schema::Object(Box::new(Object::new()))),
        ));
        ins_op = ins_op.summary(self.description.clone().unwrap_or_default());

        let mut description = "使用定义的ElasticSearch连接来执行相应的查询或命令".to_string();
        description.push_str("具体需要执行的查询与索引，需要在每个服务中进行定义。\n");
        description.push_str("Body: 主要是传递查询所需要的参数的JSON结构数据\n");

        ins_op = ins_op.description(description);

        let mut resp = Response::new("返回ApiResult结构的JSON对象。".to_owned());
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(
                RefOr::Type(Schema::Object(Box::new(Object::new()))),
                array,
            )),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));
        ins_op
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ElasticSearchPluginConfig {
    pub connection: Option<String>,
    pub server_type: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub services: Vec<ElasticSearchTargetInfo>,
}

impl ElasticSearchPluginConfig {
    fn get_target(&self, name: &str) -> Option<ElasticSearchTargetInfo> {
        let opt = self
            .services
            .iter()
            .find(|p| p.name == *name)
            .map(|f| f.to_owned());
        if opt.is_some() {
            opt
        } else {
            let cpt = name
                .split(':')
                .map(|t| t.to_owned())
                .collect::<Vec<String>>();
            if !cpt.is_empty() {
                let name_ = cpt[0].clone();
                let index_ = if cpt.len() >= 2 {
                    let cpts = cpt.clone();
                    let (_, cts) = cpts.split_at(1);
                    Some(cts.join(":"))
                } else {
                    None
                };
                let tg = ElasticSearchTargetInfo {
                    name: name_.clone(),
                    index: index_,
                    op_type: Some(name_.clone()),
                    api_type: Some("index".to_owned()),
                    ..Default::default()
                };
                Some(tg)
            } else {
                None
            }
        }
    }
}

#[allow(dead_code)]
pub struct ElasticSearchPluginService {
    namespace: String,
    conf: PluginConfig,
    elastic: Mutex<Option<ElasticSearchPluginConfig>>,
}

impl ElasticSearchPluginService {
    pub fn new(ns: &str, conf: &PluginConfig) -> Result<Self, anyhow::Error> {
        log::debug!("Plugin config load from {}", conf.config.clone());
        let t = match load_config(conf.config.clone()) {
            Ok(r) => r,
            Err(err) => {
                log::debug!("Could not load the config file: {err:?}");
                Some(ElasticSearchPluginConfig::default())
            }
        };

        Ok(Self {
            namespace: ns.to_owned(),
            conf: conf.to_owned(),
            elastic: Mutex::new(t),
        })
    }

    fn to_openapi_doc(&self, ns: &str) -> OpenApi {
        let mut openapi = OpenApi::new(self.conf.name.clone(), "0.1.0");
        let tconf = self.elastic.lock().unwrap().clone();
        if let Some(restconf) = tconf {
            for tsvc in restconf.services.iter().cloned() {
                let topic = tsvc.name.clone();
                let opt = tsvc.to_operation(false);
                let one_path = format!(
                    "/api/elasticsearch/{}/{}/{}",
                    ns,
                    self.conf.name.clone(),
                    topic,
                );
                openapi = openapi.add_path(
                    one_path.clone(),
                    PathItem::new(salvo::oapi::PathItemType::Post, opt.clone()),
                );
            }
        }
        openapi
    }

    fn create_client(
        esconf: &ElasticSearchPluginConfig,
    ) -> Result<elasticsearch::Elasticsearch, anyhow::Error> {
        let elastic: ElasticSearchPluginConfig = esconf.clone();

        let client = match elastic
            .server_type
            .clone()
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "cloud" => {
                let credentials = Credentials::Basic(
                    elastic.username.unwrap_or_default(),
                    elastic.password.unwrap_or_default(),
                );
                let trn = Transport::cloud(&elastic.connection.unwrap_or_default(), credentials)?;
                Elasticsearch::new(trn)
            }
            "nonsecurity" => {
                let transport = Transport::single_node(&elastic.connection.unwrap_or_default())?;
                Elasticsearch::new(transport)
            }
            _ => {
                let credentials = Credentials::Basic(
                    elastic.username.unwrap_or_default(),
                    elastic.password.unwrap_or_default(),
                );
                let url = Url::parse(&elastic.connection.unwrap_or_default())?;
                let conn_pool = SingleNodeConnectionPool::new(url);
                let trn = TransportBuilder::new(conn_pool)
                    .cert_validation(elasticsearch::cert::CertificateValidation::None)
                    .disable_proxy()
                    .auth(credentials)
                    .build()?;
                Elasticsearch::new(trn)
            }
        };

        Ok(client)
    }

    /**
     * 用于管理ES中的索引
     * 如：创建索引，更新索引，删除索引，查询ES中所有的索引
     */
    async fn index_handle(
        elastic: &Elasticsearch,
        target: &ElasticSearchTargetInfo,
        args: Vec<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        let index = target.index.clone().unwrap_or_default();
        log::info!(
            "index {index}, op: {}",
            target.op_type.clone().unwrap_or_default()
        );
        let result = match target.op_type.clone().unwrap_or_default().as_str() {
            "list" => {
                if index.is_empty() {
                    elastic
                        .cat()
                        .indices(elasticsearch::cat::CatIndicesParts::None)
                        .format("json")
                        .send()
                        .await
                } else {
                    let indexes = vec![index.as_str()];
                    elastic
                        .cat()
                        .indices(elasticsearch::cat::CatIndicesParts::Index(&indexes))
                        .format("json")
                        .send()
                        .await
                }
            }
            "create" => {
                let jsonbody = args[0].clone();
                elastic
                    .indices()
                    .create(elasticsearch::indices::IndicesCreateParts::Index(&index))
                    .body(jsonbody)
                    .send()
                    .await
            }
            "update_aliases" => {
                let jsonbody = args[0].clone();
                let indexes = vec![index.as_str()];
                let alias_name = json_path_get_string(&jsonbody, "$.aliases.name");
                elastic
                    .indices()
                    .put_alias(elasticsearch::indices::IndicesPutAliasParts::IndexName(
                        &indexes,
                        &alias_name,
                    ))
                    .body(jsonbody)
                    .send()
                    .await
            }
            "update_mappings" => {
                let jsonbody = args[0].clone();
                let indexes = vec![index.as_str()];
                elastic
                    .indices()
                    .put_mapping(elasticsearch::indices::IndicesPutMappingParts::Index(
                        &indexes,
                    ))
                    .body(jsonbody)
                    .send()
                    .await
            }
            "update_settings" => {
                let jsonbody = args[0].clone();
                let indexes = vec![index.as_str()];
                elastic
                    .indices()
                    .put_settings(elasticsearch::indices::IndicesPutSettingsParts::Index(
                        &indexes,
                    ))
                    .body(jsonbody)
                    .send()
                    .await
            }
            "setting" => {
                let indexes = vec![index.as_str()];
                elastic
                    .indices()
                    .get(elasticsearch::indices::IndicesGetParts::Index(&indexes))
                    .send()
                    .await
            }
            "delete" => {
                let indexes = vec![index.as_str()];
                elastic
                    .indices()
                    .delete(elasticsearch::indices::IndicesDeleteParts::Index(&indexes))
                    .send()
                    .await
            }
            _ => {
                return Err(anyhow!("Unknow op type"));
            }
        };

        match result {
            Ok(ts) => match ts.json::<Value>().await {
                Ok(rt) => Ok(Some(rt)),
                Err(err) => Err(anyhow!(err)),
            },
            Err(err) => Err(anyhow!(err)),
        }
    }

    /**
     * 用于管理ES中具体某个索引的操作
     * 如，往索引中加入文档，删除文档，修改，以及查询，按ID获取等操作。
     */
    async fn document_handle(
        elastic: &Elasticsearch,
        target: &ElasticSearchTargetInfo,
        args: Vec<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        if args.is_empty() {
            return Err(anyhow!("No args provided."));
        }

        let reqjson = if let Ok(t) = template_eval(
            &target.request_body.clone().unwrap_or_default(),
            json!({"args": args}),
        ) {
            match serde_json::from_str::<Value>(&t) {
                Ok(vt) => vt,
                Err(err) => {
                    log::info!("error to parse the template {err}");
                    args[0].clone()
                }
            }
        } else {
            args[0].clone()
        };

        let id = json_path_get_string(
            &reqjson,
            &format!("$.{}", target.id_field.clone().unwrap_or_default()),
        );
        let index = target.index.clone().unwrap_or_default();

        // log::warn!("request json: {reqjson:?}");
        // log::info!("index {index}, id: {id}, op: {}", target.op_type.clone().unwrap_or_default());

        let result = match target.op_type.clone().unwrap_or_default().as_str() {
            "get" => {
                let part = GetParts::IndexId(&index, &id);
                elastic.get(part).send().await
            }
            "merge" => {
                let part = GetParts::IndexId(&index, &id);
                match elastic.get(part).send().await {
                    Ok(resp) => {
                        if resp.status_code() == StatusCode::NOT_FOUND {
                            let part = CreateParts::IndexId(&index, &id);
                            elastic.create(part).body(reqjson).send().await
                        } else {
                            let uppart = UpdateParts::IndexId(&index, &id);
                            elastic
                                .update(uppart)
                                .body(serde_json::json!({"doc": reqjson }))
                                .send()
                                .await
                        }
                    }
                    Err(err) => {
                        return Err(anyhow!("Error to get by Index ID. {err}"));
                    }
                }
            }
            "create" => {
                let part = CreateParts::IndexId(&index, &id);
                elastic.create(part).body(reqjson).send().await
            }
            "update" => {
                let uppart = UpdateParts::IndexId(&index, &id);
                elastic
                    .update(uppart)
                    .body(serde_json::json!({"doc": reqjson }))
                    .send()
                    .await
            }
            "delete" => {
                let dlpart = DeleteParts::IndexId(&index, &id);
                elastic.delete(dlpart).send().await
            }
            "search" => {
                let indexes = vec![index.as_str()];
                let searchpart = SearchParts::Index(indexes.as_slice());
                elastic.search(searchpart).body(reqjson).send().await
            }
            _ => {
                return Err(anyhow!("Unknow op type"));
            }
        };

        match result {
            Ok(ts) => match ts.json::<Value>().await {
                Ok(rt) => Ok(Some(rt)),
                Err(err) => Err(anyhow!(err)),
            },
            Err(err) => Err(anyhow!(err)),
        }
    }

    async fn invoke_request(
        elastic: &ElasticSearchPluginConfig,
        target: &ElasticSearchTargetInfo,
        args: Vec<Value>,
    ) -> Result<Option<Value>, anyhow::Error> {
        // let client = self.create_client()?;
        // client.async_search().delete("parts").send().await;
        // let client = Self::create_client(&self)?;
        // client.delete(DeleteParts::IndexId(&target.target_url)).send().await;
        let client = Self::create_client(elastic)?;
        let mut map = serde_json::Map::new();
        for arg in args {
            if let Value::Object(mut mp) = arg {
                map.append(&mut mp);
            }
        }

        let ctxval = Value::Object(map);
        let apitype = target.api_type.clone().unwrap_or("document".to_owned());
        let res = match apitype.as_str() {
            "index" => Self::index_handle(&client, target, vec![ctxval]).await?,
            _ => Self::document_handle(&client, target, vec![ctxval]).await?,
        };
        // log::warn!("response {}", serde_json::to_string_pretty(&res.clone().unwrap_or(Value::Null)).unwrap());
        match res {
            Some(ts) => {
                if let Some(ret_validate) = target.return_validate.clone() {
                    if !ret_validate.is_empty() {
                        let ret_val = json_path_get(&ts, &ret_validate);
                        if ret_val.is_none() {
                            return Err(anyhow!("return data was not validated"));
                        }
                    }
                }

                if let Some(rt) = target.return_data.clone() {
                    let ret = json_path_get(&ts, &rt);
                    Ok(ret)
                } else {
                    Ok(Some(ts))
                }
            }
            None => Ok(None),
        }
    }
}

unsafe impl Send for ElasticSearchPluginService {}

unsafe impl Sync for ElasticSearchPluginService {}

impl RxPluginService for ElasticSearchPluginService {
    fn invoke_direct_query(
        &self,
        config: Value,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        let elastic = match self.elastic.lock().unwrap().clone() {
            Some(t) => t,
            None => {
                return Box::pin(async move { Err(anyhow!("No elasticsearch config provided.")) })
            }
        };
        // log::warn!("direct query this {config:?}");
        let targ = match serde_json::from_value::<ElasticSearchTargetInfo>(config) {
            Ok(t) => t,
            Err(err) => {
                return Box::pin(async move {
                    Err(anyhow!(
                        "Target info can not to be convert by this method provided. {err}"
                    ))
                });
            }
        };

        Box::pin(async move {
            match Self::invoke_request(&elastic, &targ, args).await {
                Ok(res) => Ok(option_value_to_vec_value(&res)),
                Err(err) => Err(err),
            }
        })
    }

    fn invoke_return_option(
        &self,
        uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>, anyhow::Error>> + Send>> {
        let elastic = match self.elastic.lock().unwrap().clone() {
            Some(t) => t,
            None => {
                return Box::pin(async move { Err(anyhow!("No elasticsearch config provided.")) })
            }
        };

        let method = uri.method.clone();

        // log::info!("Method call {}#{method}", uri.url());

        let targ = match elastic.get_target(&method) {
            Some(t) => t,
            None => {
                return Box::pin(async move {
                    Err(anyhow!(
                        "No target {method} to be defined by this method provided."
                    ))
                });
            }
        };

        Box::pin(async move {
            match Self::invoke_request(&elastic, &targ, args).await {
                Ok(res) => Ok(res),
                Err(err) => Err(err),
            }
        })
    }

    fn invoke_return_vec(
        &self,
        uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Value>, anyhow::Error>> + Send>> {
        let elastic = match self.elastic.lock().unwrap().clone() {
            Some(t) => t,
            None => {
                return Box::pin(async move { Err(anyhow!("No elasticsearch config provided.")) })
            }
        };

        let method = uri.method.clone();

        // log::info!("Method call {}#{method}", uri.url());

        let targ = match elastic.get_target(&method) {
            Some(t) => t,
            None => {
                return Box::pin(async move {
                    Err(anyhow!(
                        "No target {method} to be defined by this method provided."
                    ))
                });
            }
        };

        Box::pin(async move {
            match Self::invoke_request(&elastic, &targ, args).await {
                Ok(res) => Ok(option_value_to_vec_value(&res)),
                Err(err) => Err(err),
            }
        })
    }

    fn invoke_return_page(
        &self,
        _uri: InvokeUri,
        _ctx: Arc<Mutex<InvocationContext>>,
        _args: Vec<Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Page<Value>, anyhow::Error>> + Send>> {
        Box::pin(async move { Err(anyhow!("Not implemented")) })
    }

    fn get_config(&self) -> Option<Value> {
        match serde_json::to_value(self.elastic.lock().unwrap().clone()) {
            Ok(t) => Some(t),
            Err(err) => {
                log::debug!("Convert to json with error: {err:?}");
                None
            }
        }
    }

    fn parse_config(&self, val: &Value) -> Result<(), anyhow::Error> {
        match serde_json::from_value::<ElasticSearchPluginConfig>(val.to_owned()) {
            Ok(t) => {
                self.elastic.lock().unwrap().replace(t);
                Ok(())
            }
            Err(err) => {
                log::info!("Parse JSON value to config with error: {err:?}");
                Err(anyhow!(err))
            }
        }
    }

    fn save_config(&self, conf: &PluginConfig) -> Result<(), anyhow::Error> {
        let path: PathBuf = conf.config.clone().into();
        chimes_store_core::service::starter::save_config(
            &self.elastic.lock().unwrap().clone(),
            path,
        )
    }

    fn get_metadata(&self) -> Vec<chimes_store_core::service::sdk::MethodDescription> {
        let elastic = self.elastic.lock().unwrap().clone();
        let mut veclist = vec![];
        if let Some(els) = elastic {
            for svc in els.services.iter().cloned() {
                veclist.push(MethodDescription {
                    uri: format!(
                        "{}://{}/{}",
                        self.conf.protocol.clone(),
                        self.namespace.clone(),
                        self.conf.name.clone()
                    ),
                    name: svc.name.clone(),
                    func: None,
                    params_vec: true,
                    params1: vec![],
                    params2: None,
                    response: vec![],
                    return_page: false,
                    return_vec: false,
                })
            }
        }

        veclist
    }

    fn get_openapi(&self, ns: &str) -> Box<dyn std::any::Any> {
        Box::new(self.to_openapi_doc(ns))
    }
}
