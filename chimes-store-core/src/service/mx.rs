use crate::{
    dbs::probe::{ColumnInfo, KeyColumnInfo, TableInfo},
    utils::get_multiple_rbatis,
};

use super::{sdk::MxProbeService, starter::MxStoreService};

impl MxProbeService for MxStoreService {
    /**
     * 列出所有的对象
     */
    async fn probe_schema(
        &self,
        schema: &str,
    ) -> Result<Vec<crate::dbs::probe::TableInfo>, anyhow::Error> {
        let rb = get_multiple_rbatis(&self.0.db_url);
        TableInfo::load_tables(rb, schema).await
    }

    /**
     * 列出所有的对象
     */
    async fn probe_one_table(
        &self,
        schema: &str,
        tbl: &str,
    ) -> Result<Option<crate::dbs::probe::TableInfo>, anyhow::Error> {
        let rb = get_multiple_rbatis(&self.0.db_url);
        TableInfo::find_one_table(rb, schema, tbl).await
    }
    /**
     * 列出所有的结构
     */
    async fn probe_table(
        &self,
        schema: &str,
        tbl: &str,
    ) -> Result<Vec<crate::dbs::probe::ColumnInfo>, anyhow::Error> {
        let rb = get_multiple_rbatis(&self.0.db_url);
        match ColumnInfo::load_columns(rb, schema, tbl).await {
            Ok(t) => Ok(t),
            Err(err) => {
                log::info!("Error {}", err);
                Err(err)
            }
        }
    }

    async fn probe_table_keys(
        &self,
        schema: &str,
        tbl: &str,
    ) -> Result<Vec<crate::dbs::probe::KeyColumnInfo>, anyhow::Error> {
        let rb = get_multiple_rbatis(&self.0.db_url);
        match KeyColumnInfo::load_table_pkeys(rb, schema, tbl).await {
            Ok(t) => Ok(t),
            Err(err) => {
                log::info!("Error {}", err);
                Err(err)
            }
        }
    }
}
