use anyhow::anyhow;
use chimes_store_core::{service::queue::SyncTaskQueue, utils::ApiResult};
use salvo::{
    handler,
    writing::{Json, Text},
    Depot, Request,
};
use serde_json::{json, Value};
use std::thread;

#[handler]
pub async fn proxy_test(_depot: &mut Depot, _req: &mut Request) -> Text<String> {
    println!("current thread id: [{:?}]", thread::current().id());
    match reqwest::Client::new()
        .get("https://www.baidu.com")
        .send()
        .await
    {
        Ok(rs) => match rs.text().await {
            Ok(text) => Text::Html(text),
            Err(err) => Text::Json(json!({"errmsg": anyhow!(err).to_string()}).to_string()),
        },
        Err(err) => Text::Json(json!({"errmsg": anyhow!(err).to_string()}).to_string()),
    }
    //})
}

#[handler]
pub async fn taskqueue_count_get(
    _depot: &mut Depot,
    _req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let counter = SyncTaskQueue::get_mut().get_count();
    Json(ApiResult::ok(counter))
}
