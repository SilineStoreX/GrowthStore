use std::{
    fs::{create_dir_all, File},
    future::Future,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use super::{fetch::SyncReader, SyncTaskDefinition, SyncVariable};
use crate::proc::excel::ExcelFileParser;
use anyhow::anyhow;
use chimes_store_core::service::{queue::SyncTaskQueue, sched::JobInvoker};
use chimes_store_core::utils::global_data::{bool_from_str, i64_from_str, u8_from_str};
use chimes_store_utils::{algorithm::snowflake_id, template::json_path_get};
use csv::Terminator;
use rbatis::rbdc::Uuid;
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileFetchParam {
    pub parser: String,               // 解释器，如jsonl，csv， xml, json，excel等
    pub parse_param: Option<Value>,   // 解释器参数，可选，内部的数据内容由解释器维护
    pub remove_source: bool,          // 文件被处理后，删除文件
    pub backup_dir: Option<String>,   // 文件被处理后，实际上会被移动到该目录
    pub filename_pattern: String,     // 文件名的模式，满足该模式的文件名为有效文件
    pub recuise: bool,                // 是否遍历子目录
    pub id_generator: Option<String>, // ID字段的生成方法，snowflak_id, uuid, 缺省为snowflake_id
    pub id_field: Option<String>,     // ID字段的属性名称，缺省为id
}

/**
 * FileUriReader
 * 提供用于基于本地目录的文件发现，以及处理的读取器
 * FileUriReader根据FileFetchParam中定义的参数来进行发现，以及读取相应的数据结构，最终返回Value对象
 * FileFetchParam定义了文件发现以及处理的指令
 */
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct FileUriReader {
    namespace: String,
    variables: Vec<SyncVariable>,
    task: SyncTaskDefinition,
    param: FileFetchParam,
    #[serde(skip_serializing)]
    pattern: Arc<Regex>,
}

impl FileUriReader {
    pub fn new(ns: &str, vars: &[SyncVariable], task: &SyncTaskDefinition) -> Self {
        let pt = serde_json::from_str::<FileFetchParam>(
            &task.source_request.clone().unwrap_or_default(),
        )
        .unwrap_or_default();
        let filepatt = Regex::new(&pt.filename_pattern.clone())
            .unwrap_or(Regex::new("__BAD_FILE_NAME_[.]").unwrap());
        Self {
            namespace: ns.to_owned(),
            variables: vars.to_vec(),
            task: task.clone(),
            param: pt,
            pattern: Arc::new(filepatt),
        }
    }

    fn fetch_subdir(&self, path: &PathBuf, sourcepath: &str) -> Result<(), anyhow::Error> {
        match std::fs::read_dir(path) {
            Ok(dir) => {
                for entry in dir {
                    let entry = entry?;
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        self.fetch_subdir(&entry_path, sourcepath)?;
                    } else if let Some(fl) = entry_path.file_name() {
                        let filename = fl.to_string_lossy().to_string();
                        log::info!("filename: {filename}");
                        if self.pattern.is_match(&filename) {
                            log::error!("process file {entry_path:?}");
                            self.process_file(&entry_path, sourcepath)?;
                        }
                    }
                }
                Ok(())
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    fn create_parser(&self) -> Result<Box<dyn FileParser>, anyhow::Error> {
        let parser = self.param.parser.clone();
        log::error!("Parser {parser}");
        match parser.as_str() {
            "jsonl" => {
                log::error!("create a jsonl file parser");
                Ok(Box::new(JsonlFileParser(
                    self.task.task_id.clone(),
                    self.param.clone(),
                )))
            }
            "json" => {
                log::error!("create a json file parser");
                Ok(Box::new(JSONFileParser(
                    self.task.task_id.clone(),
                    self.param.clone(),
                )))
            }
            "csv" => {
                log::error!("create a csv file parser");
                Ok(Box::new(CSVFileParser(
                    self.task.task_id.clone(),
                    self.param.clone(),
                )))
            }
            "excel" => {
                log::error!("create a excel file parser");
                Ok(Box::new(ExcelFileParser(
                    self.task.task_id.clone(),
                    self.param.clone(),
                )))
            }
            _ => Ok(Box::new(MockFileParser)),
        }
    }

    fn process_file(&self, file: &Path, sourcepath: &str) -> Result<(), anyhow::Error> {
        let parser = self.create_parser()?;
        parser.prcoess_file(file, &self.param)?;
        log::error!("here to remove sources");
        if self.param.remove_source {
            if let Some(backup) = self.param.backup_dir.clone() {
                if !backup.is_empty() {
                    let fullname = file.to_string_lossy().to_string().replace("\\", "/");
                    let repback = backup.replace("\\", "/");
                    let newfilename = fullname.replace(sourcepath, &repback);
                    let newfilepath: PathBuf = newfilename.clone().into();
                    if let Some(parent) = newfilepath.parent() {
                        if let Err(err) = create_dir_all(parent) {
                            log::debug!("create dir all failed {err}");
                        }
                    }

                    if let Err(err) = std::fs::copy(fullname, &newfilename) {
                        log::error!("could not copy to backup file {newfilename}. Error {err}");
                    }
                }
            }
            if let Err(err) = std::fs::remove_file(file) {
                log::error!("Could not remove file {err:?}.");
            }
        }
        Ok(())
    }

    /**
     * 遍历目录
     */
    fn fetch_dir(&self) -> Result<(), anyhow::Error> {
        let fileuri = self.task.source_uri.clone().unwrap_or_default();
        match Url::parse(&fileuri) {
            Ok(u) => match u.to_file_path() {
                Ok(path) => {
                    let sourcepath = path.to_string_lossy().to_string().replace("\\", "/");
                    match std::fs::read_dir(path) {
                        Ok(dir) => {
                            for entry in dir {
                                let entry = entry?;
                                let entry_path = entry.path();
                                if entry_path.is_dir() {
                                    log::error!("enter path {entry_path:?}");
                                    if self.param.recuise {
                                        self.fetch_subdir(&entry_path, &sourcepath)?;
                                    }
                                } else if let Some(fl) = entry_path.file_name() {
                                    let filename = fl.to_string_lossy().to_string();
                                    log::error!("filename: {filename}, {}", self.pattern.as_str());
                                    if self.pattern.is_match(&filename) {
                                        log::error!("process file {entry_path:?}");
                                        self.process_file(&entry_path, &sourcepath)?;
                                    }
                                }
                            }
                            Ok(())
                        }
                        Err(err) => Err(anyhow!(err)),
                    }
                }
                Err(_) => Err(anyhow!("Not a validate file URL")),
            },
            Err(err) => Err(anyhow!(err)),
        }
    }
}

pub struct FileUriReaderJob {
    inner: FileUriReader,
}

impl JobInvoker for FileUriReaderJob {
    fn exec(&self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>> {
        let mut urireader = self.inner.clone();
        Box::pin(async move {
            log::debug!("start to fetch the records in file...");
            if let Err(err) = urireader.fetch().await {
                log::error!("error for reader : {err}");
                Err(err)
            } else {
                Ok(())
            }
        })
    }

    fn get_invoke_uri(&self) -> String {
        format!(
            "synctask://{}/{}",
            self.inner.namespace.clone(),
            self.inner.task.task_id.clone()
        )
    }

    fn get_invoke_params(&self) -> Option<Value> {
        self.inner
            .task
            .params
            .clone()
            .map(|s| serde_json::from_str::<Value>(&s).unwrap_or(Value::Null))
    }

    fn get_description(&self) -> String {
        self.inner.task.task_desc.clone().unwrap_or_default()
    }
}

impl SyncReader for FileUriReader {
    fn interval_second(&self) -> Option<u64> {
        self.task.interval_second.map(|s| s as u64)
    }

    fn cron_express(&self) -> Option<String> {
        self.task.cron_express.clone()
    }

    fn fetch(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), anyhow::Error>> + Send>>
    {
        let cpt = self.clone();
        Box::pin(async move { cpt.fetch_dir() })
    }

    fn to_invoker(&self) -> Box<dyn chimes_store_core::service::sched::JobInvoker + Send + Sync> {
        Box::new(FileUriReaderJob {
            inner: self.clone(),
        })
    }
}

pub trait FileParser {
    fn add_to_queue(
        &self,
        task_id: &str,
        id_field: &Option<String>,
        id_generate: &Option<String>,
        it: &mut Value,
    ) -> Result<(), anyhow::Error> {
        let id_field = id_field.clone().unwrap_or("id".to_owned());
        let id_generator = id_generate.clone().unwrap_or("None".to_owned());

        let val = match id_generator.to_lowercase().as_str() {
            "uuid" => Value::String(Uuid::new().to_string().to_ascii_lowercase()),
            "snowflake" => json!(snowflake_id()),
            "snowflake_id" => json!(snowflake_id()),
            _ => Value::Null,
        };

        if !it.is_null() {
            it.as_object_mut().and_then(|f| f.insert(id_field, val));
        }

        let taskname = json_path_get(it, "$.name")
            .map(|t| match t {
                Value::String(text) => Some(text),
                _ => serde_json::to_string(&t)
                    .map(|s| Some(s.to_owned()))
                    .unwrap_or(None),
            })
            .unwrap_or(None);

        let subject = json_path_get(it, "$.subject")
            .map(|t| match t {
                Value::String(text) => Some(text),
                _ => serde_json::to_string(&t)
                    .map(|s| Some(s.to_owned()))
                    .unwrap_or(None),
            })
            .unwrap_or(None);

        // log::error!("{}, Value: {}", &self.0, serde_json::to_string(&tv).unwrap_or_default());
        SyncTaskQueue::get_mut().push_task(task_id, &taskname, &subject, it, 1)
    }

    fn prcoess_file(&self, file: &Path, param: &FileFetchParam) -> Result<(), anyhow::Error>;
}

pub struct MockFileParser;

impl FileParser for MockFileParser {
    fn prcoess_file(&self, _file: &Path, _param: &FileFetchParam) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

pub struct JsonlFileParser(String, FileFetchParam);

impl FileParser for JsonlFileParser {
    fn prcoess_file(&self, filepath: &Path, _param: &FileFetchParam) -> Result<(), anyhow::Error> {
        let file = File::open(filepath)?;
        let mut reader = BufReader::new(file);

        let mut buf = String::new();
        loop {
            buf.clear();
            let num_bytes_read = match reader.read_line(&mut buf) {
                Ok(bs) => bs,
                Err(err) => {
                    return Err(anyhow!(err));
                }
            };

            // log::error!("read line: {num_bytes_read}");

            if num_bytes_read == 0 {
                return Ok(());
            }

            if let Ok(mut tv) = serde_json::from_str::<Value>(&buf) {
                self.add_to_queue(&self.0, &self.1.id_field, &self.1.id_generator, &mut tv)?;
            }
        }
    }
}

pub struct JSONFileParser(String, FileFetchParam);

impl FileParser for JSONFileParser {
    fn prcoess_file(&self, filepath: &Path, _param: &FileFetchParam) -> Result<(), anyhow::Error> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        let jsonpath_ = match self.1.parse_param.clone() {
            Some(pt) => match pt {
                Value::String(jsonpath) => jsonpath.to_owned(),
                Value::Object(mp) => {
                    if let Some(jpval) = mp.get("jsonpath") {
                        match jpval {
                            Value::String(jpth) => jpth.to_owned(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            },
            None => String::new(),
        };

        let ret: Value = serde_json::from_reader(reader)?;
        let rets = if jsonpath_.is_empty() {
            match ret {
                Value::Array(ts) => ts.to_owned(),
                _ => vec![ret],
            }
        } else if let Some(ts) = json_path_get(&ret, &jsonpath_) {
            match ts {
                Value::Array(xs) => xs.to_owned(),
                _ => vec![ts],
            }
        } else {
            vec![]
        };

        for mut it in rets {
            // add this into queue
            self.add_to_queue(&self.0, &self.1.id_field, &self.1.id_generator, &mut it)?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CSVFileParseParam {
    #[serde(default)]
    #[serde(deserialize_with = "i64_from_str")]
    pub buffer_capactiy: Option<i64>,

    #[serde(default)]
    #[serde(deserialize_with = "u8_from_str")]
    pub comment: Option<u8>,

    #[serde(default)]
    #[serde(deserialize_with = "u8_from_str")]
    pub delimiter: Option<u8>,

    #[serde(default)]
    #[serde(deserialize_with = "bool_from_str")]
    pub double_quote: Option<bool>,

    #[serde(default)]
    #[serde(deserialize_with = "u8_from_str")]
    pub escape: Option<u8>,

    #[serde(default)]
    #[serde(deserialize_with = "bool_from_str")]
    pub flexible: Option<bool>,

    #[serde(default)]
    #[serde(deserialize_with = "u8_from_str")]
    pub quote: Option<u8>,

    #[serde(default)]
    #[serde(deserialize_with = "bool_from_str")]
    pub quoting: Option<bool>,

    #[serde(default)]
    #[serde(deserialize_with = "bool_from_str")]
    pub has_headers: Option<bool>,
    pub terminator: Option<String>,
    pub trim: Option<String>,
    pub columns: Vec<String>, // 必须要给出来，用于转换成为JSON对象。
}

pub struct CSVFileParser(String, FileFetchParam);

impl FileParser for CSVFileParser {
    fn prcoess_file(&self, filepath: &Path, _param: &FileFetchParam) -> Result<(), anyhow::Error> {
        let csvparam = serde_json::from_value::<CSVFileParseParam>(
            self.1.parse_param.clone().unwrap_or_default(),
        )
        .unwrap_or_default();
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        let mut builder = csv::ReaderBuilder::new();

        builder
            .ascii()
            .buffer_capacity(csvparam.buffer_capactiy.unwrap_or(1024000) as usize)
            .comment(csvparam.comment)
            .escape(csvparam.escape);

        if let Some(delimiter) = csvparam.delimiter {
            builder.delimiter(delimiter);
        }

        if let Some(double_quote) = csvparam.double_quote {
            builder.double_quote(double_quote);
        }

        if let Some(flexible) = csvparam.flexible {
            builder.flexible(flexible);
        }

        if let Some(quote) = csvparam.quote {
            builder.quote(quote);
        }

        if let Some(quoting) = csvparam.quoting {
            builder.quoting(quoting);
        }

        if let Some(has_headers) = csvparam.has_headers {
            builder.has_headers(has_headers);
        }

        if let Some(terminator) = csvparam.terminator {
            if terminator == *"CRLF" {
                builder.terminator(Terminator::CRLF);
            } else if !terminator.is_empty() {
                let c = terminator.as_bytes()[0];
                builder.terminator(Terminator::Any(c));
            }
        }

        if let Some(trim) = csvparam.trim {
            match trim.as_str() {
                "All" => builder.trim(csv::Trim::All),
                "Fields" => builder.trim(csv::Trim::Fields),
                "Headers" => builder.trim(csv::Trim::Headers),
                "None" => builder.trim(csv::Trim::None),
                _ => builder.trim(csv::Trim::None),
            };
        }

        let mut csvreder = builder.from_reader(reader);

        let cols = csvparam.columns.len();
        for it in csvreder.records() {
            match it {
                Ok(rec) => {
                    let mut objmap = Map::new();
                    let mincols = cols.min(rec.len());
                    for fi in 0..mincols {
                        let colname = csvparam.columns[fi].clone();
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

                        match rec.get(fi) {
                            Some(tv) => {
                                let gval = match column_type.as_str() {
                                    "long" | "integer" => {
                                        json!(tv.parse::<i64>().unwrap_or_default())
                                    }
                                    "number" => {
                                        json!(tv.parse::<f64>().unwrap_or_default())
                                    }
                                    "boolean" => {
                                        let bl = tv == "true" || tv == "yes";
                                        Value::Bool(bl)
                                    }
                                    _ => Value::String(tv.to_owned()),
                                };
                                objmap.insert(column_name, gval);
                            }
                            None => {
                                objmap.insert(column_name, Value::Null);
                            }
                        }
                    }

                    let mut it = Value::Object(objmap);
                    self.add_to_queue(&self.0, &self.1.id_field, &self.1.id_generator, &mut it)?;
                }
                Err(err) => {
                    log::warn!("error for read csv {err}");
                }
            }
        }

        Ok(())
    }
}
