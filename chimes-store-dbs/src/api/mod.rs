use salvo::Router;
pub mod crud;
pub mod redis;

pub fn get_salvo_service_router() -> Vec<Router> {
    vec![
        Router::with_path("object/<ns>/<name>/select/<id>").post(crud::select),
        Router::with_path("object/<ns>/<name>/select/<id>").get(crud::select),
        Router::with_path("object/<ns>/<name>/find_one").post(crud::find_one),
        Router::with_path("object/<ns>/<name>/insert").post(crud::insert),
        Router::with_path("object/<ns>/<name>/update").post(crud::update),
        Router::with_path("object/<ns>/<name>/upsert").post(crud::upsert),
        Router::with_path("object/<ns>/<name>/save_batch").post(crud::save_batch),
        Router::with_path("object/<ns>/<name>/delete").post(crud::delete),
        Router::with_path("object/<ns>/<name>/delete_by").post(crud::delete_by),
        Router::with_path("object/<ns>/<name>/update_by").post(crud::update_by),
        Router::with_path("object/<ns>/<name>/query").post(crud::query),
        Router::with_path("object/<ns>/<name>/paged_query").post(crud::paged_query),
        Router::with_path("query/<ns>/<name>/search").post(crud::query_search),
        Router::with_path("query/<ns>/<name>/paged_search").post(crud::query_paged_search),
        Router::with_path("redis/<ns>/redis/get").get(redis::redis_get_object),
        Router::with_path("redis/<ns>/redis/list").get(redis::redis_get_vec),
        Router::with_path("redis/<ns>/redis/page").get(redis::redis_get_page),
        Router::with_path("redis/<ns>/redis/set").post(redis::redis_set_infinit),
        Router::with_path("redis/<ns>/redis/del").post(redis::redis_del_infinit),
        Router::with_path("redis/<ns>/redis/keys").get(redis::redis_keys_vec),
        Router::with_path("redis/<ns>/redis/flushall").post(redis::redis_flushall),
        Router::with_path("redis/<ns>/redis/delexp").post(redis::redis_delexp),
        Router::with_path("object/<ns>/<name>/test").post(crud::test),
    ]
}

pub fn get_management_redis_service_routers() -> Vec<Router> {
    vec![
        Router::with_path("/<ns>/redis/get").get(redis::redis_get_object),
        Router::with_path("/<ns>/redis/list").get(redis::redis_get_vec),
        Router::with_path("/<ns>/redis/page").get(redis::redis_get_page),
        Router::with_path("/<ns>/redis/set").post(redis::redis_set_infinit),
        Router::with_path("/<ns>/redis/del").post(redis::redis_del_infinit),
        Router::with_path("/<ns>/redis/keys").get(redis::redis_keys_vec),
        Router::with_path("/<ns>/redis/flushall").post(redis::redis_flushall),
        Router::with_path("/<ns>/redis/delexp").post(redis::redis_delexp),
    ]
}
