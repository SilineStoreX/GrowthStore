use rbatis::rbdc::db::Driver;
use substring::Substring;

pub fn get_sql_driver(url: &str) -> impl Driver {
    let driver: Box<dyn Driver> = match url
        .find("://")
        .map(|f| url.substring(0, f))
        .unwrap_or("mysql")
    {
        "postgres" => Box::new(rbdc_pg::PostgresDriver {}),
        "sqlite" => Box::new(rbdc_sqlite::SqliteDriver {}),
        "jdbc:sqlserver" => Box::new(rbdc_mssql::MssqlDriver {}),
        "sqlserver" => Box::new(rbdc_mssql::MssqlDriver {}),
        "mssql" => Box::new(rbdc_mssql::MssqlDriver {}),
        // "oracle" => Box::new(rbdc_oracle::driver::OracleDriver {}),
        "tao+ws" => Box::new(rbdc_tdengine::driver::TaosDriver {}),
        _ => Box::new(rbdc_mysql::MysqlDriver {}),
    };
    driver
}

pub fn get_driver_name(driver: &rbatis::RBatis) -> &str {
    driver.driver_type().unwrap_or("mysql")
}

pub fn get_load_column_sql(driver: &rbatis::RBatis) -> &str {
    match get_driver_name(driver) {
        "postgres" => {
            "SELECT table_schema as table_schema, table_name as table_name,  column_name as column_name,
        column_default as column_default, udt_name as orginal_type, ordinal_position as ordinal_position, character_maximum_length as character_maximum_length, 
        is_nullable as is_nullable, numeric_precision as numeric_precision, numeric_scale as numeric_scale
        FROM INFORMATION_SCHEMA.COLUMNS WHERE case when ? = '' then table_schema like '%' else table_schema = ? end and table_name = ? order by ORDINAL_POSITION ASC"
        },
        "mssql" => {
            "SELECT table_schema as table_schema, table_name as table_name,  column_name as column_name,
            column_default as column_default, data_type as orginal_type, ordinal_position as ordinal_position, character_maximum_length as character_maximum_length, 
            is_nullable as is_nullable, numeric_precision as numeric_precision, numeric_scale as numeric_scale
            FROM INFORMATION_SCHEMA.COLUMNS WHERE table_schema like (case when ? = '' then '%' else CONCAT(?, '%') end) and table_name = ? order by ORDINAL_POSITION ASC"
        },
        _ => {
            "SELECT table_schema as table_schema, table_name as table_name,  column_name as column_name,
        column_default as column_default, data_type as orginal_type, ordinal_position as ordinal_position, character_maximum_length as character_maximum_length, 
        is_nullable as is_nullable, numeric_precision as numeric_precision, numeric_scale as numeric_scale
        FROM INFORMATION_SCHEMA.COLUMNS WHERE case when ? = '' then table_schema like '%' else table_schema = ? end and table_name = ? order by ORDINAL_POSITION ASC" 
        }
    }
}

pub fn get_load_tables_sql(_driver: &rbatis::RBatis) -> &str {
    r#"SELECT table_catalog as table_catalog, table_schema as table_schema, table_type as table_type, 
    table_name as table_name
    FROM INFORMATION_SCHEMA.TABLES WHERE table_schema = ?"#
}

pub fn get_load_one_tables_sql(_driver: &rbatis::RBatis) -> &str {
    r#"SELECT table_catalog as table_catalog, table_schema as table_schema, table_type as table_type, 
    table_name as table_name
    FROM INFORMATION_SCHEMA.TABLES WHERE table_schema = ? and table_name = ?"#
}


pub fn get_load_table_pkey_sql(_driver: &rbatis::RBatis) -> &str {
    r#"select table_schema as table_schema, table_name as table_name, column_name as column_name, ordinal_position as ordinal_position
    from INFORMATION_SCHEMA.key_column_usage where  table_schema = ? and table_name = ? and (CONSTRAINT_NAME = 'PRIMARY' OR CONSTRAINT_NAME like '%_pk' OR CONSTRAINT_NAME like '%_pkey') order by ORDINAL_POSITION ASC "#
}

pub fn get_update_field_value_present(driver: &rbatis::RBatis, field: &str, field_type: &str) -> String {
    match get_driver_name(driver) {
        "postgres" => {
            format!("{field} = ?::{field_type}")
        },
        _ => {
            format!("{field} = ?")
        }
    }
}

pub fn get_insert_field_value_present(driver: &rbatis::RBatis, _field: &str, field_type: &str) -> String {
    match get_driver_name(driver) {
        "postgres" => {
            format!("?::{field_type}")
        },
        _ => {
            "?".to_string()
        }
    }
}