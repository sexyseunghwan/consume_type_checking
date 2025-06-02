/*
Author      : Seunghwan Shin
Create date : 2025-06-02
Description : Program that identifies which consumption type the words entered by the user are classified

History     : 2025-06-02 Seunghwan Shin       # [v.1.0.0] first create
*/
mod common;
use common::*;

mod utils_module;
use utils_module::logger_utils::*;

mod controller;
use controller::main_controller::*;

mod repository;

mod service;
use service::query_service::*;
use service::es_query_service::*;

mod model;

mod entity;

mod config;

#[tokio::main]
async fn main() {
    set_global_logger();
    dotenv().ok();
    
    info!("CONSUME Check Program Start");

    let query_service: QueryServicePub = QueryServicePub::new();
    let es_query_service: EsQueryServicePub = EsQueryServicePub::new();
    let main_controller: MainController<QueryServicePub, EsQueryServicePub> =
        MainController::new(query_service, es_query_service);
    
    match main_controller.main_task().await {
        Ok(_) => info!("CONSUME Check Program End"),
        Err(e) => error!("{e:?}"),
    }
}
