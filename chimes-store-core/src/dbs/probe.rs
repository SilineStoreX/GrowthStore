use chimes_dbs_factory::{get_load_column_sql, get_load_one_tables_sql, get_load_table_pkey_sql, get_load_tables_sql};
use itertools::Itertools;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};

use crate::config::Column;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TableInfo {
    pub table_catalog: Option<String>,
    pub table_schema: Option<String>,
    pub table_type: Option<String>,
    pub table_name: Option<String>,
}

unsafe impl Send for TableInfo {}
unsafe impl Sync for TableInfo {}

impl TableInfo {
    pub async fn load_tables(rb: &RBatis, table_schema: &str) -> anyhow::Result<Vec<TableInfo>> {
        // log::info!("TS: {}, TN: {}", table_schema.clone(), table_name.clone());
        let rb_args = vec![rbs::to_value!(table_schema)];
        let sql = get_load_tables_sql(rb);
        match rb.query_decode::<Vec<TableInfo>>(
            sql,
            rb_args).await {
            Ok(rt) => Ok(rt),
            Err(err) => {
                Err(anyhow::Error::new(err))
            }
        }
    }

    pub async fn find_one_table(
        rb: &RBatis,
        table_schema: &str,
        table_name: &str,
    ) -> anyhow::Result<Option<TableInfo>> {
        // log::info!("TS: {}, TN: {}", table_schema.clone(), table_name.clone());
        let rb_args = vec![rbs::to_value!(table_schema), rbs::to_value!(table_name)];
        let sql = get_load_one_tables_sql(rb);
        match rb.query_decode::<Vec<TableInfo>>(
            sql,
            rb_args).await {
            Ok(rt) => {
                if rt.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(rt[0].to_owned()))
                }
            },
            Err(err) => {
                Err(anyhow::Error::new(err))
            }
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ColumnInfo {
    pub table_schema: Option<String>,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
    pub prop_name: Option<String>,
    pub column_default: Option<String>,
    pub orginal_type: Option<String>,
    pub data_type: Option<String>,
    pub ordinal_position: Option<i64>,
    pub character_maximum_length: Option<i64>,
    pub is_nullable: Option<String>,
    pub numeric_precision: Option<i64>,
    pub numeric_scale: Option<i64>,
}

unsafe impl Send for ColumnInfo {}
unsafe impl Sync for ColumnInfo {}

impl ColumnInfo {
    //#[sql("SELECT table_schema, table_name,  column_name, column_type, column_comment, column_key,
    //        column_default, orginal_type, ordinal_position, character_maximum_length, is_nullable, numeric_precision, numeric_scale,
    //        FROM INFORMATION_SCHEMA.COLUMNS WHERE table_schema = ? and table_name = ?")]
    pub async fn load_columns(rb: &RBatis, ts: &str, tn: &str) -> anyhow::Result<Vec<Self>> {
        let rb_args = vec![rbs::to_value!(ts), rbs::to_value!(ts), rbs::to_value!(tn)];
        // rb.update_by_wrapper(table, w, skips);
        let sql = get_load_column_sql(rb);
        match rb.query_decode::<Vec<ColumnInfo>>(
            sql,
            rb_args).await {
            Ok(rs) =>{
                 Ok(rs.into_iter().map(|mut f| {
                    f.data_type = Some(Self::convert_to_type(&f.orginal_type.clone().unwrap_or_default()));
                    f
                }).collect_vec())
            },
            Err(err) => {
                Err(anyhow::Error::new(err))
            }
        }
    }

    pub fn convert_to_type(type_name: &str) -> String {
        match type_name {
            "int" | "int8" | "int4" | "integer" | "long" | "unsigned int" | "smallint"
            | "bigint" => "integer".to_owned(),
            "date" => "date".to_owned(),
            "time" => "time".to_owned(),
            "datetime" | "timestamp" => "datetime".to_owned(),
            "bool" | "bit" | "boolean" | "tinyint" => "bool".to_owned(),
            "json" => "json".to_owned(),
            "blob" | "binary" | "clob" | "longblob" | "longclob" => "binary".to_owned(),
            _ => "string".to_owned(),
        }
    }
}

impl From<ColumnInfo> for Column {
    fn from(val: ColumnInfo) -> Column {
        Column {
            field_name: val.column_name.clone().unwrap_or_default(),
            prop_name: val.column_name.clone(),
            title: val.column_name.clone(),
            col_length: val.character_maximum_length,
            col_type: val.data_type.clone(),
            field_type: val.orginal_type.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct KeyColumnInfo {
    pub table_schema: Option<String>,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
    pub ordinal_position: Option<i64>,
}

unsafe impl Send for KeyColumnInfo {}
unsafe impl Sync for KeyColumnInfo {}

impl KeyColumnInfo {
    pub async fn load_table_pkeys(rb: &RBatis, ts: &str, tn: &str) -> anyhow::Result<Vec<Self>> {
        let rb_args = vec![rbs::to_value!(ts), rbs::to_value!(tn)];
        // rb.update_by_wrapper(table, w, skips);
        let sql = get_load_table_pkey_sql(rb);
        match rb.query_decode(
            sql,
            rb_args).await {
            Ok(rs) => Ok(rs),
            Err(err) => {
                Err(anyhow::Error::new(err))
            }
        }
    }
}
