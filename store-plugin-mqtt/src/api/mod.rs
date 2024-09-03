use std::sync::{Arc, Mutex};

use chimes_store_core::service::invoker::JwtFromDepot;
use chimes_store_core::service::{
    invoker::InvocationContext, sdk::InvokeUri, starter::MxStoreService,
};
use chimes_store_core::utils::ApiResult;
use salvo::handler;
use salvo::{writing::Json, Depot, Request};
use serde_json::Value;


// 使用定义的MQTT连接来发送MQTT消息
// MQTT的发送是一个标准的协义，发送过程中，无法太多的配置
// 如此，我们通过MQTT的publish方法来来发送时，主要是使用topic，qos，以及payload来作为参数即可。
// 因此，我们定义了发送接口如下：
// Query：我们可以使用QueryString来传递topic和qos，如果Query在转成JSON时，包含了值，则会将其作为参数传过去
// Body: 主要是传递payload。
//       当然也可以传递topic和qos
// Body有几种情况：
//      1、Body是一个JSON数组，且该数据的长度大于2，则第一个JSON为topic，第二个为qos，第三个为payload
//      2、Body是一个JSON数组，且该数据的长度等于2，则第一个JSON为{topic, qos}，第二个为payload
//      3、Body是一个JSON对象，它必须包含了topic, qos, payload三个属性（注qos为可选，如果没有传，则为0）
#[handler]
pub async fn publish_mqtt_request(
    depot: &mut Depot,
    req: &mut Request,
) -> Json<ApiResult<Option<Value>>> {
    let ns = req.param::<String>("ns").unwrap();
    let name = req.param::<String>("name").unwrap();
    let method = req.param::<String>("method").unwrap();
    let method_rep = method.replace('+', "/");
    let uri = format!("mqtt://{ns}/{name}#{method_rep}");
    let mut args = vec![];

    match req.parse_body::<Value>().await {
        Ok(tt) => {
            match tt.clone() {
                Value::Array(mut tms) => {
                    args.append(&mut tms);
                },
                Value::Object(_tm) => {
                    args.push(tt);
                }
                _ => {
                    return Json(ApiResult::error(
                        400,
                        "No payload provided",
                    ));
                }
            };
        }
        Err(err) => {
            log::info!("Could not parse the body as json value {:?}", err);
            args.push(Value::Null);
        }
    }

    if let Ok(invoke_uri) = InvokeUri::parse(&uri) {
        match MxStoreService::get_plugin_service(&invoke_uri.url_no_method()) {
            Some(pls) => {
                let ctx = Arc::new(Mutex::new(InvocationContext::from_depot(depot)));
                match pls.invoke_return_option(invoke_uri, ctx, args).await {
                    Ok(ret) => Json(ApiResult::ok(ret)),
                    Err(err) => Json(ApiResult::error(
                        500,
                        &format!("Runtime exception: {:?}", err),
                    )),
                }
            }
            None => Json(ApiResult::error(
                404,
                &format!("Not-Found for plugin-service {}", uri),
            )),
        }
    } else {
        Json(ApiResult::error(
            404,
            &format!("Could not parse URI {}", uri),
        ))
    }
}
