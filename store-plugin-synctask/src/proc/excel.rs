use std::path::Path;

use super::filefetch::{FileFetchParam, FileParser};
use anyhow::anyhow;
use calamine::{open_workbook, Data, Range, Reader, Xls, Xlsx};
use chimes_store_core::utils::global_data::i64_from_str;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExcelFileParseParam {
    pub sheetname: Option<String>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub sheet_no: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub start_rows: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub start_col: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub end_col: Option<i64>,
    pub columns: Vec<String>, // 必须要给出来，用于转换成为JSON对象。
}

pub struct ExcelFileParser(pub(crate) String, pub(crate) FileFetchParam);

impl ExcelFileParser {
    fn parse_xls(
        &self,
        filepath: &Path,
        xlsparam: &ExcelFileParseParam,
    ) -> Result<(), anyhow::Error> {
        let mut xlsbook: Xls<_> = open_workbook(filepath)?;
        let sheet = match xlsparam.sheetname.clone() {
            Some(stname) => xlsbook.worksheet_range(&stname)?,
            None => {
                let sht = xlsparam.sheet_no.unwrap_or_default();
                if let Some(sx) = xlsbook.worksheet_range_at(sht as usize) {
                    sx?
                } else {
                    return Err(anyhow!("Not found sheet"));
                }
            }
        };

        self.parse_range(sheet, xlsparam)
    }

    fn parse_xlsx(
        &self,
        filepath: &Path,
        xlsparam: &ExcelFileParseParam,
    ) -> Result<(), anyhow::Error> {
        let mut xlsbook: Xlsx<_> = open_workbook(filepath)?;
        let sheet = match xlsparam.sheetname.clone() {
            Some(stname) => xlsbook.worksheet_range(&stname)?,
            None => {
                let sht = xlsparam.sheet_no.unwrap_or_default();
                if let Some(sx) = xlsbook.worksheet_range_at(sht as usize) {
                    sx?
                } else {
                    return Err(anyhow!("Not found sheet"));
                }
            }
        };

        self.parse_range(sheet, xlsparam)
    }

    fn parse_range(
        &self,
        sheet: Range<Data>,
        xlsparam: &ExcelFileParseParam,
    ) -> Result<(), anyhow::Error> {
        let start_row = xlsparam.start_rows.unwrap_or_default() as u32;
        let start_col = xlsparam.start_col.unwrap_or_default() as u32;
        let end_col = xlsparam.end_col.unwrap_or_default() as u32;

        let mut row = start_row;
        loop {
            let mut onerow = vec![];
            for col in start_col..end_col {
                let colval = match sheet.get_value((row, col)) {
                    Some(tv) => match tv {
                        Data::Bool(bl) => Value::Bool(bl.to_owned()),
                        Data::Int(i) => json!(i),
                        Data::Float(fl) => json!(fl),
                        Data::String(text) => {
                            if text.trim().is_empty() {
                                Value::Null
                            } else {
                                let bytes = text.as_bytes();
                                match String::from_utf8(bytes.to_vec()) {
                                    Ok(rtext) => Value::String(rtext),
                                    Err(_) => Value::Null,
                                }
                                // text.escape_unicode().to_string();
                                // Value::String(text.to_owned())
                            }
                        }
                        Data::DateTime(dt) => {
                            if dt.as_f64() < 0.1 {
                                Value::Null
                            } else {
                                Value::String(dt.to_string())
                            }
                        }
                        Data::DateTimeIso(text) => {
                            if text.trim().is_empty() {
                                Value::Null
                            } else {
                                let bytes = text.as_bytes();
                                match String::from_utf8(bytes.to_vec()) {
                                    Ok(rtext) => Value::String(rtext),
                                    Err(_) => Value::Null,
                                }
                                // Value::String(text.to_owned())
                            }
                        }
                        Data::DurationIso(text) => {
                            if text.trim().is_empty() {
                                Value::Null
                            } else {
                                let bytes = text.as_bytes();
                                match String::from_utf8(bytes.to_vec()) {
                                    Ok(rtext) => Value::String(rtext),
                                    Err(_) => Value::Null,
                                }
                                // Value::String(text.to_owned())
                            }
                        }
                        _ => Value::Null,
                    },
                    None => Value::Null,
                };

                onerow.push(colval);
            }

            if self.is_null_row(&onerow) {
                break;
            }

            self.convert_and_add(&onerow, xlsparam)?;
            row += 1;
        }

        Ok(())
    }

    fn is_null_row(&self, row: &[Value]) -> bool {
        row.iter().all(|p| p.is_null())
    }

    fn convert_and_add(
        &self,
        row: &[Value],
        xlsparam: &ExcelFileParseParam,
    ) -> Result<(), anyhow::Error> {
        let mut objmap = Map::new();
        let mincols = xlsparam.columns.len().min(row.len());
        for fi in 0..mincols {
            let colname = xlsparam.columns[fi].clone();
            let (column_name, column_type) = {
                let cols_ = colname.split(":").collect::<Vec<&str>>();
                if cols_.is_empty() {
                    log::warn!("column is not defined.");
                    continue;
                } else if cols_.len() >= 2 {
                    let col_name = cols_[0].to_string();
                    let col_type = cols_[1].to_string();
                    (col_name, col_type)
                } else {
                    let col_name = cols_[0].to_string();
                    (col_name, "String".to_owned())
                }
            };

            match row.get(fi) {
                Some(tv) => {
                    let gval = match column_type.as_str() {
                        "long" | "integer" => match tv {
                            Value::String(text) => {
                                json!(text.parse::<i64>().unwrap_or_default())
                            }
                            _ => tv.to_owned(),
                        },
                        "number" => match tv {
                            Value::String(text) => {
                                json!(text.parse::<f64>().unwrap_or_default())
                            }
                            _ => tv.to_owned(),
                        },
                        "boolean" => match tv {
                            Value::String(text) => Value::Bool(text == "true" || text == "yes"),
                            _ => tv.to_owned(),
                        },
                        _ => tv.to_owned(),
                    };
                    objmap.insert(column_name, gval);
                }
                None => {
                    objmap.insert(column_name, Value::Null);
                }
            }
        }

        let mut it = Value::Object(objmap);
        self.add_to_queue(&self.0, &self.1.id_field, &self.1.id_generator, &mut it)
    }
}

impl FileParser for ExcelFileParser {
    fn prcoess_file(&self, filepath: &Path, _param: &FileFetchParam) -> Result<(), anyhow::Error> {
        let xlsparam = serde_json::from_value::<ExcelFileParseParam>(
            self.1.parse_param.clone().unwrap_or_default(),
        )
        .unwrap_or_default();
        if filepath
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xls"))
        {
            self.parse_xls(filepath, &xlsparam)
        } else if filepath
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xlsx"))
        {
            self.parse_xlsx(filepath, &xlsparam)
        } else {
            Err(anyhow!("Not suppoprt format"))
        }
    }
}
