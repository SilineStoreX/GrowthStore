use std::{collections::HashMap, convert::Infallible, fs::File, io::{BufRead, Seek, SeekFrom}, path::PathBuf, time::Duration};

use chimes_store_core::{
    service::{
        perfs::PerformanceSummary,
        registry::SchemaRegistry,
        sched::{JobStateInfo, JobStateQuery, SchedulerHolder},
        starter::MxStoreService,
    }, utils::{build_path, get_multiple_rbatis_async, ApiResult, GlobalConfig}
};
use chimes_store_utils::file::get_current_dir;
use futures_util::StreamExt;
use itertools::Itertools;
use rbatis::Page;
// use salvo::{handler, http::ReqBody, oapi::{endpoint, extract::JsonBody, RequestBody}, writing::Json};
use salvo::{fs::NamedFile, handler, oapi::extract::JsonBody, prelude::*};
use serde_json::Value;
use tokio::time::interval;
use tokio_stream::{wrappers::IntervalStream};

use crate::utils::{backwardreader::BackwardsReader, ChimesPerformanceInfo, LogViewRequest, ManageApiResult};

#[endpoint]
pub async fn performance_get() -> Json<ManageApiResult<ChimesPerformanceInfo>> {
    let nss = MxStoreService::get_namespaces();
    let mut dburls = vec![];
    let mut dburlmap: HashMap<String, Vec<String>> = HashMap::new();
    for ns_ in nss {
        if let Some(mss) = MxStoreService::get(&ns_) {
            let dburl = mss.get_db_url();
            dburls.push(dburl.clone());
            match dburlmap.entry(dburl) {
                std::collections::hash_map::Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().push(ns_);
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(vec![ns_]);
                }
            }
        }
    }

    dburls.sort();
    dburls.dedup();

    match ChimesPerformanceInfo::get_performance_info() {
        Ok(mut st) => {
            for dburl in dburls {
                let rb_ = get_multiple_rbatis_async(&dburl).await;
                if let Ok(pool) = rb_.get_pool() {
                    let mut s = pool.state().await;
                    // log::error!("State: {s:?}");
                    if let Some(vs) = dburlmap.get(&dburl) {
                        s.insert(
                            rbs::value::Value::String("namespace".to_owned()),
                            rbs::value::Value::String(vs.join(",")),
                        );
                    }
                    st.connections.insert(dburl, s);
                }
            }
            // log::error!("{}", serde_json::to_string(&st).unwrap_or_default());
            Json(ManageApiResult::ok(st))
        }
        Err(err) => Json(ManageApiResult::<ChimesPerformanceInfo>::error(
            500,
            &format!("Could not get performance {err:?}"),
        )),
    }
}

#[handler]
pub async fn performance_summary() -> Json<ApiResult<Vec<PerformanceSummary>>> {
    let summary = SchemaRegistry::get_mut().get_performance_summary();
    Json(ApiResult::ok(summary))
}

#[handler]
pub async fn task_summary(query: JsonBody<JobStateQuery>) -> Json<ApiResult<Page<JobStateInfo>>> {
    let fut = SchedulerHolder::get().query_states(query.0.clone());
    match fut.await {
        Ok(t) => Json(ApiResult::ok(t)),
        Err(err) => {
            log::info!("error to query the task state info {err}.");
            Json(ApiResult::error(500, &err.to_string()))
        }
    }
}


fn sse_response_line(line: &str) -> Result<SseEvent, Infallible> {
    Ok(SseEvent::default().text(line.to_string()))
}

#[allow(dead_code)]
fn sse_response_multi_line(lines: Vec<String>) -> Vec<Result<SseEvent, Infallible>> {
    lines.iter().map(|l| Ok(SseEvent::default().text(l))).collect()
}


#[handler]
pub async fn list_logs(depot: &mut Depot) -> Json<ApiResult<Vec<String>>> {
    if depot.jwt_auth_state() != JwtAuthState::Authorized {
        return Json(ApiResult::<Vec<String>>::error(401, "Unauthorized"));
    }

    let current_path = match get_current_dir() {
        Ok(cpath) => cpath,
        Err(err) => {
            return Json(ApiResult::<Vec<String>>::error(401, &format!("error to read {err}")));
        }
    };

    let logdir = match build_path(current_path, "logs") {
        Ok(logpath) => logpath,
        Err(err) => {
            return Json(ApiResult::<Vec<String>>::error(401, &format!("error to read {err}")));
        }
    };

    log::info!("current dir {logdir:?}");

    match std::fs::read_dir(logdir) {
        Ok(readir) => {
            let files = readir.into_iter().flat_map(|ft| {
                match ft {
                    Ok(fl) => {
                        fl.file_name().into_string().map(Some).unwrap_or(None)
                    },
                    Err(err) => {
                        log::error!("error to read dir {err}");
                        None
                    }
                }
            }).collect_vec();

            return Json(ApiResult::ok(files));
        },
        Err(err) => {
            return Json(ApiResult::<Vec<String>>::error(401, &format!("error to fetch dir {err}")));
        }
    }
}

#[handler]
pub async fn download_logs(depot: &mut Depot, req: &mut Request, res: &mut Response) {
    if depot.jwt_auth_state() != JwtAuthState::Authorized {
        res.render(Json(ApiResult::<Vec<String>>::error(401, "Unauthorized")));
        return;
    }

    let file = match  req.query::<String>("file") {
        Some(t) => {
            if t.is_empty() {
                res.render(Json(ApiResult::<Vec<String>>::error(400, "The file did not be provided.")));
                return;
            }
            t
        },
        None => {
            res.render(Json(ApiResult::<Vec<String>>::error(400, "The file did not be provided.")));
            return;
        }
    };

    

    let current_path = match get_current_dir() {
        Ok(cpath) => cpath,
        Err(err) => {
            res.render(Json(ApiResult::<Vec<String>>::error(401, &format!("error to read {err}"))));
            return;
        }
    };

    let logfile = match build_path(current_path, "logs") {
        Ok(logpath) => logpath.join(file.clone()),
        Err(err) => {
            res.render(Json(ApiResult::<Vec<String>>::error(401, &format!("error to read {err}"))));
            return;
        }
    };

    NamedFile::builder(logfile)
        .attached_name(format!("{file}"))
        .send(req.headers(), res)
        .await;
}

/**
 * 读取日志
 */
#[handler]
pub async fn readlogs(depot: &mut Depot,
                      req: &mut Request,
                      res: &mut Response) {

    if depot.jwt_auth_state() != JwtAuthState::Authorized {
        res.render(Json(ApiResult::<Value>::error(401, &"Unauthorized".to_string())));
        return;
    }
    let logview = req.parse_body::<LogViewRequest>().await
                                                    .map(Some)
                                                    .unwrap_or(req.parse_queries::<LogViewRequest>().map(Some).unwrap_or(None))
                                                    .unwrap_or_default();
    let interval_time = logview.interal.unwrap_or(3) as u64;
    let num_lines = logview.reserves.unwrap_or(50) as usize;
    let logfile  = GlobalConfig::config().logfile.unwrap_or("nohup.out".to_string());
    let filename = logview.logfile.clone().unwrap_or("logs".to_string());
    let current_path = get_current_dir().unwrap();
    
    let readfilename: Result<PathBuf, anyhow::Error> = if filename == *"nohup" {
        build_path(current_path, "nohup.out")
    } else {
        let logpath = build_path(current_path.clone(), "logs").unwrap_or(current_path.clone());
        let logfilename: PathBuf = logfile.into();
        let logext = logfilename
                .clone()
                .extension()
                .unwrap()
                .to_str()
                .map(|f| f.to_string())
                .unwrap_or("log".to_string());
        let logfilename_part = logfilename
                .file_stem()
                .unwrap()
                .to_str()
                .map(|f| f.to_string())
                .unwrap();
        
        let namelogs = if filename == *"logs" {
            format!("{logfilename_part}_rCURRENT.{logext}")
        } else {
            format!("{logfilename_part}-trace_rCURRENT.{logext}")
        };

        build_path(logpath, namelogs)
    };

    match readfilename {
        Err(err) => {
            res.render(Json(ApiResult::<Value>::error(500, &err.to_string())));
        },
        Ok(tsfile) => {
            let mut lines: Vec<String> = Vec::with_capacity(num_lines);
            let event_stream = {
                let mut file = match File::open(tsfile) {
                    Ok(f) => f,
                    Err(err) => {
                        return res.render(Json(ApiResult::<Value>::error(500, &err.to_string())));
                    }
                };
                let _ = file.seek(SeekFrom::End(0)).is_ok();
                // let mut bufread = BufReader::new(file);
                let mut buf = String::new();
                let mut reader = std::io::BufReader::new(file);
                let mut br = BackwardsReader::new(num_lines, &mut reader);

                br.read_all_lines(&mut lines);
                                
                let interval = interval(Duration::from_secs(interval_time));
                let stream = IntervalStream::new(interval);
                stream.map(move |_| {
                    buf.clear();
                    let mut stlines: Vec<String> = Vec::new();
                    while let Ok(tc) = reader.read_line(&mut buf) {
                        if !buf.is_empty() {
                            stlines.push(buf.clone());
                        }
                        buf.clear();
                        if tc == 0 {
                            break;
                        }
                    }

                    sse_response_line(&stlines.join("\n"))
                })
            };

            let tal = futures_util::stream::iter(lines).map(|l| sse_response_line(&l)).chain(event_stream);
            
            salvo::sse::stream(res, tal);
        }
    }
}
