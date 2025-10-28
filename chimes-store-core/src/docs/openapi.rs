use std::collections::HashMap;

use crate::{
    config::{QueryObject, ServerConfig, StoreObject},
    service::starter::MxStoreService,
};
use salvo::oapi::{
    schema, security::ApiKeyValue, Array, Content, Object, OpenApi, Operation, Parameter, PathItem,
    RefOr, RequestBody, Response, Schema, SchemaType, SecurityScheme, Server,
};
use serde_json::{json, Value};

pub trait ToSchemaObjects {
    fn to_schema(&self, ns: &str) -> HashMap<String, schema::Schema>;
    fn to_paths(
        &self,
        ns: &str,
        schemas: &HashMap<String, schema::Schema>,
    ) -> HashMap<String, PathItem>;
}

pub trait ToOpenApiDoc {
    fn to_openapi_doc(&self, conf: &ServerConfig) -> salvo::oapi::OpenApi;
}

pub trait ToMCPTools {
    fn to_mcp_tools(&self, ns: &str) -> Result<Vec<Value>, anyhow::Error>;
}

fn to_api_result_schema(t: RefOr<Schema>, array: bool) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "status",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "message",
        Object::new().schema_type(schema::BasicType::String),
    );

    if array {
        apiresult = apiresult.property("data", Array::new().items(t)); // OLD: t.to_array()
    } else {
        apiresult = apiresult.property("data", t);
    }
    apiresult = apiresult.property(
        "timestamp",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    schema::Schema::Object(Box::new(apiresult))
}

fn to_schema_operation(name: &str, qot: bool) -> Operation {
    let mut ins_op = Operation::new();
    ins_op = ins_op.description(format!("获取{name}的相应方法的请求参数与返回值的JSONSchema表示。"));
    let mut resp = if qot {
        Response::new("返回该对象方法所需要的JSONSchema。对于存储对象返回的包括有主键（PK）的Schema，以及INSERT/Update时的Schema，和查询时响应的Schema。对于动态查询的Schema不包含在内。")
    } else {
        Response::new("返回该自定义查询执行方法时所需要的JSONSchema。对于自定义查询返回的包括有固定的自定义参数，以及响应时的Schema。")
    };

    resp = resp.add_content(
        "application/json",
        Content::new(to_api_result_schema(RefOr::Type(
            schema::Schema::Object(Box::new(Object::new())),
        ), false)),
    );
    ins_op = ins_op.add_response("200", RefOr::Type(resp));
    ins_op
}

fn to_api_result_page_schema(t: RefOr<Schema>) -> schema::Schema {
    let mut apiresult = Object::new();
    apiresult = apiresult.property(
        "total",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "page_no",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "page_size",
        Object::new().schema_type(schema::BasicType::Integer),
    );
    apiresult = apiresult.property(
        "do_count",
        Object::new().schema_type(schema::BasicType::Integer),
    );

    apiresult = apiresult.property("records", Array::new().items(t)); // OLD: t.to_array()
    to_api_result_schema(
        RefOr::Type(schema::Schema::Object(Box::new(apiresult))),
        false,
    )
}

fn to_add_query_condition(t: Schema) -> schema::Schema {
    let mut ci = Object::new()
        .property(
            "field",
            Object::new()
                .schema_type(schema::BasicType::String)
                .description("字段名称"),
        )
        .property(
            "op",
            Object::new()
                .schema_type(schema::BasicType::String)
                .description("字段名称"),
        )
        .property(
            "value",
            Object::new()
                .schema_type(schema::BasicType::Object)
                .description("字段名称"),
        )
        .property(
            "value2",
            Object::new()
                .schema_type(schema::BasicType::Object)
                .description("字段名称"),
        );
    let xci = ci
        .clone()
        .property(
            "and",
            Object::new()
                .schema_type(schema::BasicType::Array)
                .description("AND查询条伯列表"),
        )
        .property(
            "or",
            Object::new()
                .schema_type(schema::BasicType::Array)
                .description("OR查询条伯列表"),
        );

    ci = ci.property("and", xci.clone().to_array().description("AND查询条伯列表"));
    ci = ci.property("or", xci.clone().to_array().description("OR查询条伯列表"));

    let oi = Object::new()
        .property(
            "field",
            Object::new()
                .schema_type(schema::BasicType::String)
                .description("排序的字段"),
        )
        .property(
            "sort_asc",
            Object::new()
                .schema_type(schema::BasicType::Boolean)
                .description("升序或降序"),
        );
    let pg = Object::new()
        .property(
            "current",
            Object::new()
                .schema_type(schema::BasicType::Integer)
                .description("当前页码"),
        )
        .property(
            "size",
            Object::new()
                .schema_type(schema::BasicType::Integer)
                .description("分页记录数"),
        );

    let qc = Object::new()
        .property(
            "and",
            ci.clone()
                .to_array()
                .description("组合成AND条件字段及条件表达"),
        )
        .property(
            "or",
            ci.clone()
                .to_array()
                .description("组合成OR条件字段及条件表达"),
        )
        .property(
            "sorts",
            oi.clone().to_array().description("需要进行排序的字段"),
        )
        .property(
            "group_by",
            oi.clone()
                .to_array()
                .description("分组查询中需要列出的字段"),
        )
        .property("paging", pg.clone().description("分页"));

    if let Schema::Object(xmp) = t {
        let mut mp = xmp.clone();
        *mp = mp.property("_cond", qc);
        schema::Schema::Object(mp)
    } else {
        schema::Schema::Object(Box::new(qc))
    }
}

impl ToOpenApiDoc for MxStoreService {
    fn to_openapi_doc(&self, conf: &ServerConfig) -> salvo::oapi::OpenApi {
        let ns = self.get_namespace();
        let mut openapi = OpenApi::new(&ns, conf.version.clone().unwrap_or("0.1.0a".to_string()))
            .add_server(Server::new("").description("Current Server"))
            .add_server(Server::new(format!(
                "http://{}:{}",
                conf.address.clone(),
                conf.port
            )))
            // .add_path(path, item)
            // .add_schema(name, schema)
            // .add_schema(name, schema)
            .add_security_scheme(
                "Authorization",
                SecurityScheme::ApiKey(salvo::oapi::security::ApiKey::Header(ApiKeyValue::new(
                    "Authorization",
                ))),
            );
        openapi = openapi.add_schema(
            "QueryCondition",
            to_add_query_condition(schema::Schema::Array(Object::new().to_array())),
        );
        for sto in self.get_objects() {
            let mt = sto.to_schema(&self.get_namespace());
            for (key, val) in mt.clone() {
                openapi = openapi.add_schema(key, val);
            }

            let ht = sto.to_paths(&self.get_namespace(), &mt);
            for (key, val) in ht {
                openapi = openapi.add_path(key, val);
            }
        }

        for sto in self.get_querys() {
            let mt = sto.to_schema(&self.get_namespace());
            for (key, val) in mt.clone() {
                openapi = openapi.add_schema(key, val);
            }

            openapi = openapi.add_schema(
                "QueryCondition",
                to_add_query_condition(schema::Schema::Array(Object::new().to_array())),
            );

            let ht = sto.to_paths(&self.get_namespace(), &mt);
            for (key, val) in ht {
                openapi = openapi.add_path(key, val);
            }
        }

        for pls in self.get_plugins() {
            let nsuri = format!("{}://{}/{}", pls.protocol, ns, pls.name);
            if let Some(plsin) = Self::get_plugin_service(&nsuri) {
                if let Ok(plsopenapi) = plsin.get_openapi(&ns).downcast::<OpenApi>() {
                    // log::info!("merge the {nsuri}'s open api docs");
                    openapi = openapi.merge(*plsopenapi);
                }
            }
        }

        // openapi.add_path(path, item);
        openapi
    }
}

/**
 * StoreObject will generate some object like
 * 1. Insert Object/Update Object,
 * 2. Upsert Object/Keys Object,
 * 3. QueryCondition
 * 4. Page<>
 */
impl ToSchemaObjects for StoreObject {
    fn to_schema(&self, ns: &str) -> HashMap<String, schema::Schema> {
        let mut hash = HashMap::new();
        let mut colobj = Object::new();
        let mut keyobj = Object::new();
        for k in self.fields.clone() {
            let col_type = k
                .col_type
                .clone()
                .map(|f| match f.to_lowercase().as_str() {
                    "String" | "varchar" | "string" | "text" | "longtext" => {
                        schema::BasicType::String
                    }
                    "Integer" | "Long" | "long" | "integer" | "int" | "i64" | "u64" => {
                        schema::BasicType::Integer
                    }
                    "Float" | "Double" | "f32" | "f64" => schema::BasicType::Number,
                    "Boolean" | "Bool" | "bool" | "boolean" => schema::BasicType::Boolean,
                    "date" | "datetime" | "time" | "timestamp" => schema::BasicType::String,
                    _ => schema::BasicType::Object,
                })
                .unwrap_or(schema::BasicType::Object);
            let fmt = k
                .col_type
                .clone()
                .map(|f| match f.to_lowercase().as_str() {
                    "long" | "i64" | "u64" | "i128" | "bigint" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int64)
                    }
                    "int" | "i32" | "integer" | "u32" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int32)
                    }
                    "smallint" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int16),
                    "tinyint" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int8),
                    "bool" | "boolean" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Byte)
                    }
                    "date" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::Date),
                    "datetime" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::DateTime),
                    "decimal" | "numeric" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Decimal)
                    }
                    "float" | "f32" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Float)
                    }
                    "double" | "f64" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Double)
                    }
                    "relation" => {
                        schema::SchemaFormat::Custom(k.relation_object.unwrap_or_default())
                    }
                    _ => schema::SchemaFormat::Custom(f.clone().to_lowercase()),
                })
                .unwrap_or(schema::SchemaFormat::Custom("Unknown".to_owned()));

            let mut col_type_obj = if k.relation_array {
                Object::new()
                    .schema_type(schema::BasicType::Array)
                    .format(fmt)
                // colobj = colobj.property(k.prop_name.unwrap_or(k.field_name.clone()), );
            } else {
                Object::new().schema_type(col_type).format(fmt)
                // colobj = colobj.property(k.prop_name.unwrap_or(k.field_name.clone()), );
            };

            if k.col_length.is_some() {
                col_type_obj = col_type_obj.max_length(k.col_length.unwrap_or_default() as usize);
            }
            col_type_obj = col_type_obj.description(k.title.unwrap_or(k.field_name.clone()));
            colobj = colobj.property(
                k.prop_name.clone().unwrap_or(k.field_name.clone()),
                col_type_obj.clone(),
            );

            // log::info!("k.{}", k.field_name.clone());

            if k.pkey {
                keyobj = keyobj.property(k.prop_name.unwrap_or(k.field_name.clone()), col_type_obj);
            }
        }
        hash.insert(
            format!("object://{}/{}", ns, self.name.clone()),
            schema::Schema::Object(Box::new(colobj)),
        );
        if !keyobj.properties.is_empty() {
            hash.insert(
                format!("object://{}/{}PK", ns, self.name.clone()),
                schema::Schema::Object(Box::new(keyobj)),
            );
        }
        hash
    }

    fn to_paths(
        &self,
        ns: &str,
        schemas: &HashMap<String, schema::Schema>,
    ) -> HashMap<String, PathItem> {
        let mut hash = HashMap::new();

        let sch = schemas
            .get(&format!("object://{}/{}", ns, self.name.clone()))
            .unwrap();
        let sch_ref = RefOr::Type(sch.clone());

        let schpk = schemas.get(&format!("object://{}/{}PK", ns, self.name.clone()));
        let schpk_ref = if let Some(sch_val) = schpk {
            RefOr::Type(sch_val.clone())
        } else {
            RefOr::Type(sch.clone())
        };

        // schema
        hash.insert(
            format!("/api/object/{}/{}/schema", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Get, to_schema_operation(&self.name, true))
        );

        // insert
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", sch_ref.clone()),
        );
        ins_op = ins_op.summary(format!("对象{}的新增操作", self.name.clone()));
        ins_op = ins_op.description(format!("对表{}执行数据库的insert操作，如果表中有自增ID，则该自增ID应留空，返回值中将会包含所产生的自增ID。", self.object_name.clone()));

        let mut resp = Response::new("返回新增后的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/insert", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //update
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", sch_ref.clone()),
        );
        ins_op = ins_op.summary(format!("对象{}的更新操作", self.name.clone()));
        ins_op = ins_op.description(format!("对表{}执行数据库的update操作。作为该对象的主键必须提供。更新操作时，如果内容没有被修改，则不会对该字段进行更新。", self.object_name.clone()));

        let mut resp = Response::new("返回更新后的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/update", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //delete

        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", schpk_ref.clone()),
        );
        ins_op = ins_op.summary(format!("对象{}的删除操作", self.name.clone()));
        ins_op = ins_op.description(format!(
            "对表{}执行数据库的按主键删除操作。作为该对象的主键必须提供。主键必须提供。",
            self.object_name.clone()
        ));

        let mut resp = Response::new("返回更新后的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/delete", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //upsert
        let mut ins_op = Operation::new();
        let upsert_req = to_add_query_condition(sch.clone());
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", RefOr::Type(upsert_req.clone())),
        );
        ins_op = ins_op.summary(format!("对象{}的更新或新增操作", self.name.clone()));
        ins_op = ins_op.description(format!("对表{}执行数据库的update或insert操作。作为该对象的主键如果没有提供，则执行insert操作，否则，会根据主键执行查询，如果主键查询有对应的记录，则执行update，没有则执行insert操作。更新操作时，如果内容没有被修改，则不会对该字段进行更新。同时，如果传入的对象中，有包含_cond的QueryCondition对象，则会根据_cond所表示的查询条件来执行查询，进而判断是否执行相应的操作。", self.object_name.clone()));

        let mut resp = Response::new("返回更新后或插入的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/upsert", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //save_batch
        let mut savebatch_op = Operation::new();

        let savebatch_req = if let Schema::Object(schobj) = sch.clone() {
            Schema::Array(schobj.to_array())
        } else {
            Schema::Array(Object::with_type(schema::BasicType::Object).to_array())
        };

        savebatch_op = savebatch_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", RefOr::Type(savebatch_req.clone())),
        );
        savebatch_op =
            savebatch_op.summary(format!("对象{}的批量更新或新增操作", self.name.clone()));
        savebatch_op = savebatch_op.description(format!("对表{}执行数据库的upsert操作，接口通过POST接收对象的JSON结构，且，按照upsert的机制，数组中的元素中，可以附带有_cond的属性，用于确定对象的唯一性。作为该对象的主键如果没有提供，则执行insert操作，否则，会根据主键执行查询，如果主键查询有对应的记录，则执行update，没有则执行insert操作。更新操作时，如果内容没有被修改，则不会对该字段进行更新。同时，如果传入的对象中，有包含_cond的QueryCondition对象，则会根据_cond所表示的查询条件来执行查询，进而判断是否执行相应的操作。", self.object_name.clone()));

        let mut resp = Response::new("返回更新后或插入的对象列表");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), true)),
        );
        savebatch_op = savebatch_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/save_batch", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, savebatch_op),
        );

        // delete_by
        let mut ins_op = Operation::new();
        let upsert_req = to_add_query_condition(sch.clone());
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", RefOr::Type(upsert_req.clone())),
        );
        ins_op = ins_op.summary(format!("对象{}的批量操作", self.name.clone()));
        ins_op = ins_op.description(format!("对表{}执行数据库的delete操作。删除操作的条件由Request Body传入的QueryCondition组装而成。", self.object_name.clone()));

        let mut resp = Response::new("返回更新后或插入的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/delete_by", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //update_by
        let mut ins_op = Operation::new();
        let upsert_req = to_add_query_condition(sch.clone());
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content("application/json", RefOr::Type(upsert_req.clone())),
        );
        ins_op = ins_op.summary(format!("对象{}的按条件批量更新操作", self.name.clone()));
        ins_op = ins_op.description(format!("对表{}执行数据库的update操作。传入的对象中基础的该对象的要更新的数据，_cond为更新时的批量条件。", self.object_name.clone()));

        let mut resp = Response::new("返回受影响的记录数");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/update_by", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //select
        let mut ins_op = Operation::new().add_parameter(Parameter::new("id"));
        ins_op = ins_op.summary(format!("对象{}的按主键查询操作", self.name.clone()));
        ins_op = ins_op.description(format!(
            "对表{}执行数据库的按主键查询操作。主键通过URL中的<id>提供。",
            self.object_name.clone()
        ));

        let mut resp = Response::new("返回查询到的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/select/{{id}}", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Get, ins_op),
        );

        //find_one
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content(
                    "application/json",
                    RefOr::Type(to_add_query_condition(schema::Schema::Array(
                        Object::new().to_array(),
                    ))),
                ),
        );
        ins_op = ins_op.summary(format!("对象{}的唯一记录查询", self.name.clone()));
        ins_op = ins_op.description(format!(
            "对表{}执行数据库的按SELECT操作，返回零条或一条件记录。请求体传递QueryCondition",
            self.object_name.clone()
        ));

        let mut resp = Response::new("返回查询到的对象");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/find_one", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //query
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content(
                    "application/json",
                    RefOr::Type(to_add_query_condition(schema::Schema::Array(
                        Object::new().to_array(),
                    ))),
                ),
        );
        ins_op = ins_op.summary(format!("对象{}的列表查询", self.name.clone()));
        ins_op = ins_op.description(format!(
            "对表{}执行数据库的按SELECT操作，返回多条记录。请求体传递QueryCondition",
            self.object_name.clone()
        ));

        let mut resp = Response::new("返回查询到的对象列表");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_schema(sch_ref.clone(), false)),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/query", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        //paged_query
        let mut ins_op = Operation::new();
        ins_op = ins_op.request_body(
            RequestBody::new()
                .required(salvo::oapi::Required::True)
                .add_content(
                    "application/json",
                    RefOr::Type(to_add_query_condition(schema::Schema::Array(
                        Object::new().to_array(),
                    ))),
                ),
        );
        ins_op = ins_op.summary(format!("对象{}的唯一记录查询", self.name.clone()));
        ins_op = ins_op.description(format!("对表{}执行数据库的按SELECT操作，返回零条或一条件记录。请求体传递QueryCondition，且QueryCondition中的paging条件必须提供", self.object_name.clone()));

        let mut resp = Response::new("返回查询到的对象分页列表");
        resp = resp.add_content(
            "application/json",
            Content::new(to_api_result_page_schema(sch_ref.clone())),
        );
        ins_op = ins_op.add_response("200", RefOr::Type(resp));

        hash.insert(
            format!("/api/object/{}/{}/paged_query", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
        );

        hash
    }
}

impl ToSchemaObjects for QueryObject {
    fn to_schema(&self, ns: &str) -> HashMap<String, schema::Schema> {
        let mut hash = HashMap::new();
        let mut colobj = Object::new();
        for k in self.params.clone() {
            let col_type = k
                .col_type
                .clone()
                .map(|f| match f.to_lowercase().as_str() {
                    "String" | "varchar" | "string" | "text" | "longtext" => {
                        schema::BasicType::String
                    }
                    "Integer" | "Long" | "long" | "integer" | "int" | "i64" | "u64" => {
                        schema::BasicType::Integer
                    }
                    "Float" | "Double" | "f32" | "f64" => schema::BasicType::Number,
                    "Boolean" | "Bool" | "bool" | "boolean" => schema::BasicType::Boolean,
                    _ => schema::BasicType::Object,
                })
                .unwrap_or(schema::BasicType::Object);
            let fmt = k
                .col_type
                .clone()
                .map(|f| match f.to_lowercase().as_str() {
                    "long" | "i64" | "u64" | "i128" | "bigint" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int64)
                    }
                    "int" | "i32" | "integer" | "u32" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int32)
                    }
                    "smallint" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int16),
                    "tinyint" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::Int8),
                    "bool" | "boolean" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Byte)
                    }
                    "date" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::Date),
                    "datetime" => schema::SchemaFormat::KnownFormat(schema::KnownFormat::DateTime),
                    "decimal" | "numeric" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Decimal)
                    }
                    "float" | "f32" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Float)
                    }
                    "double" | "f64" => {
                        schema::SchemaFormat::KnownFormat(schema::KnownFormat::Double)
                    }
                    "relation" => {
                        schema::SchemaFormat::Custom(k.relation_object.unwrap_or_default())
                    }
                    _ => schema::SchemaFormat::Custom(f.clone().to_lowercase()),
                })
                .unwrap_or(schema::SchemaFormat::Custom("Unknown".to_owned()));

            let mut col_type_obj = if k.relation_array {
                Object::new()
                    .schema_type(schema::BasicType::Array)
                    .format(fmt)
                // colobj = colobj.property(k.prop_name.unwrap_or(k.field_name.clone()), );
            } else {
                Object::new().schema_type(col_type).format(fmt)
                // colobj = colobj.property(k.prop_name.unwrap_or(k.field_name.clone()), );
            };

            if k.col_length.is_some() {
                col_type_obj = col_type_obj.max_length(k.col_length.unwrap_or_default() as usize);
            }
            col_type_obj = col_type_obj.description(k.title.unwrap_or(k.field_name.clone()));
            colobj = colobj.property(k.prop_name.unwrap_or(k.field_name.clone()), col_type_obj);
        }
        let sch = schema::Schema::Object(Box::new(colobj));

        hash.insert(format!("query://{}/{}", ns, self.name.clone()), sch);
        hash
    }

    fn to_paths(
        &self,
        ns: &str,
        schemas: &HashMap<String, schema::Schema>,
    ) -> HashMap<String, PathItem> {
        let mut hash = HashMap::new();

        let sch = schemas
            .get(&format!("query://{}/{}", ns, self.name.clone()))
            .unwrap();

        hash.insert(
            format!("/api/query/{}/{}/schema", ns, self.name.clone()),
            PathItem::new(salvo::oapi::PathItemType::Get, to_schema_operation(&self.name, false))
        );

        if !self.updatable {
            //search
            let mut ins_op = Operation::new();
            ins_op = ins_op.request_body(
                RequestBody::new()
                    .required(salvo::oapi::Required::True)
                    .add_content(
                        "application/json",
                        RefOr::Type(to_add_query_condition(sch.clone())),
                    ),
            );
            ins_op = ins_op.summary(format!("自定义查询{}的列表查询", self.name.clone()));
            ins_op = ins_op.description(format!("对表{}执行数据库的按SELECT操作，返回多条记录。请求体通过_cond传递QueryCondition，其它字段为该查询所必须的固定参数", self.object_name.clone()));

            let mut resp = Response::new("返回查询到的对象列表");
            resp = resp.add_content(
                "application/json",
                Content::new(to_api_result_schema(
                    RefOr::Type(schema::Schema::Object(Box::new(Object::new()))),
                    false,
                )),
            );
            ins_op = ins_op.add_response("200", RefOr::Type(resp));

            hash.insert(
                format!("/api/query/{}/{}/search", ns, self.name.clone()),
                PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
            );
        }

        if self.pagable {
            //paged_query
            let mut ins_op = Operation::new();
            ins_op = ins_op.request_body(
                RequestBody::new()
                    .required(salvo::oapi::Required::True)
                    .add_content(
                        "application/json",
                        RefOr::Type(to_add_query_condition(sch.clone())),
                    ),
            );
            ins_op = ins_op.summary(format!("自定义查询{}的分页查询", self.name.clone()));
            ins_op = ins_op.description("根据自定义查询以及条件执行分页查询。请求体通过_cond传递QueryCondition，其它字段为该查询所必须的固定参数，且QueryCondition中的paging条件必须提供".to_string());

            let mut resp = Response::new("返回查询到的对象分页列表");
            resp = resp.add_content(
                "application/json",
                Content::new(to_api_result_page_schema(RefOr::Type(
                    schema::Schema::Object(Box::new(Object::new())),
                ))),
            );
            ins_op = ins_op.add_response("200", RefOr::Type(resp));

            hash.insert(
                format!("/api/query/{}/{}/paged_search", ns, self.name.clone()),
                PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
            );
        }

        if self.onlyone {
            //find_one
            let mut ins_op = Operation::new();
            ins_op = ins_op.request_body(
                RequestBody::new()
                    .required(salvo::oapi::Required::True)
                    .add_content(
                        "application/json",
                        RefOr::Type(to_add_query_condition(sch.clone())),
                    ),
            );
            ins_op = ins_op.summary(format!("自定义查询{}的唯一记录查询", self.name.clone()));
            ins_op = ins_op.description("根据自定义查询以及条件执行唯一查询，返回零条或一条件记录，如有多条记录，则会返回一条记录。请求体通过_cond传递QueryCondition，其它字段为该查询所必须的固定参数。".to_string());

            let mut resp = Response::new("返回查询到的对象");
            resp = resp.add_content(
                "application/json",
                Content::new(to_api_result_page_schema(RefOr::Type(
                    schema::Schema::Object(Box::new(Object::new())),
                ))),
            );
            ins_op = ins_op.add_response("200", RefOr::Type(resp));

            hash.insert(
                format!("/api/query/{}/{}/find_one", ns, self.name.clone()),
                PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
            );
        }

        if self.updatable {
            //execute
            let mut ins_op = Operation::new();
            ins_op = ins_op.request_body(
                RequestBody::new()
                    .required(salvo::oapi::Required::True)
                    .add_content(
                        "application/json",
                        RefOr::Type(to_add_query_condition(sch.clone())),
                    ),
            );
            ins_op = ins_op.summary(format!("自定义更新查询{}", self.name.clone()));
            ins_op = ins_op.description(
                "根据自定义查询以及条件执行INSERT/UPDATE/DELETE语句，返回受影响的记录数。"
                    .to_string(),
            );

            let mut resp = Response::new("返回受影响的记录数");
            resp = resp.add_content(
                "application/json",
                Content::new(to_api_result_page_schema(RefOr::Type(
                    schema::Schema::Object(Box::new(Object::new())),
                ))),
            );
            ins_op = ins_op.add_response("200", RefOr::Type(resp));

            hash.insert(
                format!("/api/query/{}/{}/execute", ns, self.name.clone()),
                PathItem::new(salvo::oapi::PathItemType::Post, ins_op),
            );
        }

        hash
    }
}

impl ToMCPTools for StoreObject {
    fn to_mcp_tools(&self, ns: &str) -> Result<Vec<Value>, anyhow::Error> {
        let hash = self.to_schema(ns);
        let uri_ = format!("object://{}/{}", ns, self.name);
        let uripk_ = format!("object://{}/{}PK", ns, self.name);
        let mut tools = vec![];
        let nx = ns.replace(".", "_");
        let def_schema = Object::new()
            .property(
                "type",
                Object::new().schema_type(SchemaType::Basic(schema::BasicType::String)),
            )
            .property("properties", Object::new());
        let param_obj = hash
            .get(&uri_)
            .map(|f| f.to_owned())
            .unwrap_or(Schema::Object(Box::new(def_schema.clone())));
        let param_obj_pk = hash
            .get(&uripk_)
            .map(|f| f.to_owned())
            .unwrap_or(Schema::Object(Box::new(def_schema)));

        if self.mcp_tool_query {
            let val = json!({
                "name": format!("object_{}_{}_list_query", nx, self.name.clone()),
                "invoke_uri": format!("{}#query", &uri_),
                "return_type": "list",
                "description": format!("根据参数条件查出满足条件的所有{}记录。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });
            tools.push(val);

            let paged_param_obj = match param_obj.clone() {
                Schema::Object(tsc) => {
                    let size_ = Object::new()
                        .description("查询分页的大小，即每页记录数")
                        .schema_type(SchemaType::Basic(schema::BasicType::Integer));
                    let curr_ = Object::new()
                        .description("查询分页的当前页码")
                        .schema_type(SchemaType::Basic(schema::BasicType::Integer));
                    let sch_obj = tsc
                        .clone()
                        .property("page_size", size_)
                        .property("page_current", curr_);
                    Schema::Object(Box::new(sch_obj))
                }
                _ => param_obj.clone(),
            };

            let paged = json!({
                "name": format!("object_{}_{}_paged_query", nx, self.name.clone()),
                "invoke_uri": format!("{}#paged_query", &uri_),
                "return_type": "paged",
                "description": format!("根据参数条件对{}进行分页查询。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": paged_param_obj
            });

            tools.push(paged);

            let findone = json!({
                "name": format!("object_{}_{}_find_one", nx, self.name.clone()),
                "invoke_uri": format!("{}#find_one", &uri_),
                "return_type": "optional",
                "description": format!("根据参数条件来查询{}中的唯一一条记录", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });

            tools.push(findone);

            let selectone = json!({
                "name": format!("object_{}_{}_select", nx, self.name.clone()),
                "invoke_uri": format!("{}#select", &uri_),
                "return_type": "optional",
                "description": format!("根据主键来查询{}其中的唯一的记录", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj_pk
            });

            tools.push(selectone);
        }

        if self.mcp_tool_update {
            let opinsert = json!({
                "name": format!("object_{}_{}_insert", nx, self.name.clone()),
                "invoke_uri": format!("{}#insert", &uri_),
                "return_type": "optional",
                "description": format!("新增{}对象记录，将参数对象进行保存，此操作为新增操作。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });
            tools.push(opinsert);

            let opupdate = json!({
                "name": format!("object_{}_{}_update", nx, self.name.clone()),
                "invoke_uri": format!("{}#update", &uri_),
                "return_type": "optional",
                "description": format!("根据参数对象中的{}主键信息来确定执行修改操作。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });

            tools.push(opupdate);

            let opupsert = json!({
                "name": format!("object_{}_{}_upsert", nx, self.name.clone()),
                "invoke_uri": format!("{}#upsert", &uri_),
                "return_type": "optional",
                "description": format!("根据参数中的{}对象的主键信息来确定执行新增或者修改操作。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });

            tools.push(opupsert);

            let opsavebatch = json!({
                "name": format!("object_{}_{}_save_batch", nx, self.name.clone()),
                "invoke_uri": format!("{}#save_batch", &uri_),
                "return_type": "optional",
                "description": format!("执行批量保存{}的操作，自动根据参数中的字段来判断是新增还是修改操作。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });

            tools.push(opsavebatch);

            let mb_param_value = match param_obj.clone() {
                Schema::Object(t) => {
                    Schema::Object(Box::new(t.description("将要被更新的字段及对应的值")))
                }
                _ => param_obj.clone(),
            };
            let mb_param_cond = match param_obj.clone() {
                Schema::Object(t) => Schema::Object(Box::new(
                    t.description("更新时的条件，将根据这些条件来对Value中所提示的属性进行更新"),
                )),
                _ => param_obj.clone(),
            };

            let mtc = Schema::Object(Box::new(
                Object::new()
                    .property("value", mb_param_value)
                    .property("cond", mb_param_cond),
            ));

            let opupdateby = json!({
                "name": format!("object_{}_{}_update_by", nx, self.name.clone()),
                "invoke_uri": format!("{}#update_by", &uri_),
                "return_type": "optional",
                "description": format!("根据参数条件批量修改{}记录中所对应的属性值。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": mtc
            });

            tools.push(opupdateby);
        }

        if self.mcp_tool_delete {
            let val = json!({
                "name": format!("object_{}_{}_delete", nx, self.name.clone()),
                "invoke_uri": format!("{}#delete", &uri_),
                "return_type": "optional",
                "description": format!("根据参数条件删除{}记录。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj_pk
            });
            tools.push(val);

            let delby = json!({
                "name": format!("object_{}_{}_delete_by", nx, self.name.clone()),
                "invoke_uri": format!("{}#delete_by", &uri_),
                "return_type": "optional",
                "description": format!("根据参数条件批量删除{}记录。", self.rest_desc.clone().unwrap_or_default()),
                "parameters": param_obj
            });

            tools.push(delby);
        }
        Ok(tools)
    }
}

impl ToMCPTools for QueryObject {
    fn to_mcp_tools(&self, ns: &str) -> Result<Vec<Value>, anyhow::Error> {
        let hash = self.to_schema(ns);
        let uri_ = format!("query://{}/{}", ns, self.name);
        let mut tools = vec![];
        let nx = ns.replace(".", "_");
        let def_schema = Object::new()
            .property(
                "type",
                Object::new().schema_type(SchemaType::Basic(schema::BasicType::String)),
            )
            .property("properties", Object::new());
        let param_obj = hash
            .get(&uri_)
            .map(|f| f.to_owned())
            .unwrap_or(Schema::Object(Box::new(def_schema)));
        if self.updatable {
            let val = json!({
                "name": format!("query_{}_{}_execute", nx, self.name.clone()),
                "invoke_uri": format!("{}#execute", &uri_),
                "return_type": "optional",
                "description": self.rest_desc.clone().unwrap_or_default(),
                "parameters": param_obj
            });
            tools.push(val);
        } else {
            let val = json!({
                "name": format!("query_{}_{}_list_search", nx, self.name.clone()),
                "invoke_uri": format!("{}#search", &uri_),
                "return_type": "list",
                "description": self.rest_desc.clone().unwrap_or_default(),
                "parameters": param_obj
            });
            tools.push(val);

            // 分页查询的Schema有所不同
            if self.pagable {
                let paged_param_obj = match param_obj {
                    Schema::Object(tsc) => {
                        let size_ = Object::new()
                            .description("查询分页的大小，即每页记录数")
                            .schema_type(SchemaType::Basic(schema::BasicType::Integer));
                        let curr_ = Object::new()
                            .description("查询分页的当前页码")
                            .schema_type(SchemaType::Basic(schema::BasicType::Integer));
                        let sch_obj = tsc
                            .clone()
                            .property("page_size", size_)
                            .property("page_current", curr_);
                        Schema::Object(Box::new(sch_obj))
                    }
                    _ => param_obj.clone(),
                };

                let paged = json!({
                    "name": format!("query_{}_{}_paged_search", nx, self.name.clone()),
                    "invoke_uri": format!("{}#paged_search", &uri_),
                    "return_type": "paged",
                    "description": format!("{}，本方法支持分页操作。", self.rest_desc.clone().unwrap_or_default()),
                    "parameters": paged_param_obj
                });

                tools.push(paged);
            }
        }

        Ok(tools)
    }
}
