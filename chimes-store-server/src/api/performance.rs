use salvo::{oapi::endpoint, writing::Json};

use crate::utils::{ChimesPerformanceInfo, ManageApiResult};

#[endpoint]
pub async fn performance_get() -> Json<ManageApiResult<ChimesPerformanceInfo>> {
    match ChimesPerformanceInfo::get_performance_info() {
        Ok(st) => Json(ManageApiResult::ok(st)),
        Err(err) => Json(ManageApiResult::<ChimesPerformanceInfo>::error(
            500,
            &format!("Could not get performance {:?}", err),
        )),
    }
}
