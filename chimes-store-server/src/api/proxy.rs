use chimes_store_dbs::utils::base32_decode;
use itertools::Itertools;
use salvo::{handler, writing::Json, Request, Response};
use serde_json::json;
use substring::Substring;

/**
 * Proxy.rs
 * 代理功能，主要实现对一些HTTP请求进行代理，用于跳过微信小程序的WebView的限制
 * 代理功能：以/proxy/开头，表示该请求将是对于对应路径的代理请求
 * /proxy/<host>/<**path>: proxy前缀后跟着的是Base36编码的HTTP路
 */
#[handler]
pub async fn proxy_request(req: &mut Request, resp: &mut Response) {
    // log::error!("Just calling the proxy_request");
    let _method = req.method();
    let uri = req.uri();
    // log::error!("{method} {uri}, Base32: {}", base32_encode(&"https://www.enjoylost.com"));

    let st_path = uri
        .path()
        .split("/")
        .filter(|p| !p.is_empty())
        .collect_vec();

    log::error!("path: {}", uri.path());
    log::error!("{st_path:?}");

    if st_path[0] == "proxy" {
        let host = st_path[1];
        let prefix = format!("/proxy/{host}/");
        let fullpath = uri.path();
        let proxy_path = fullpath.substring(prefix.len(), fullpath.len());
        let decode_bytes = base32_decode(host);
        let proxy_host = String::from_utf8_lossy(&decode_bytes);
        let proxy_fulluri = match uri.query() {
            Some(q) => {
                format!("{proxy_host}/{proxy_path}?{q}")
            }
            None => {
                format!("{proxy_host}/{proxy_path}")
            }
        };
        log::error!("Proxy URI: {proxy_fulluri}");
    }
    resp.render(Json(json!({"hello": "this is the best way"})));
}
