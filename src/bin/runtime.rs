use std::sync::Arc;

use actix_web::{ web::*, App, HttpServer, HttpResponse, http::StatusCode };
use clickhouse::Row;
use serde::{ Serialize, Deserialize };
use uuid::Uuid;

macro_rules! try_or_cry {
    (@impl warn $internals:tt) => { ::log::warn! $internals };
    (@impl error $internals:tt) => { ::log::error! $internals };
    (@impl $internals:tt) => { ::log::error! $internals };
    ($(@ $level:ident ->)? $result:expr  $(; $extra_data:expr)?) => {{
        match $result {
            Ok(value) => value,
            Err(e) => {
                try_or_cry!(@impl $($level)? ("Error: {}", e));
                $(::log::info!("Data: {:?}", $extra_data);)?
                return HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body("Internal Server Error");
            }
        }
    }};
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
struct ErrorRow {
    #[serde(with = "clickhouse::serde::uuid")]
    err_id: Uuid,
    service: String,
    subservice: String,
    error_message: String,
    error_data_json: String,
    #[serde(with = "clickhouse::serde::time::datetime")]
    timestamp: time::OffsetDateTime,
}

#[derive(Clone)]
struct ClickhouseState {
    client: Arc<clickhouse::Client>,
}

#[derive(Debug, Clone, Deserialize)]
struct Service {
    service: String,
    subservice: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    message: String,
}

#[actix_web::post("/log/{service}/{subservice}")]
async fn log(
    clickhouse: Data<ClickhouseState>,
    service_path: Path<Service>,
    message: Option<Query<Message>>,
    data: Bytes,
) -> HttpResponse {
    // Normalize JSON by deserializing and serializing it
    let data: serde_json::Value = try_or_cry!(@warn -> serde_json::from_slice(&data); data);
    let data = try_or_cry!(@warn -> serde_json::to_string(&data); data);

    let Service { service, subservice } = service_path.clone();
    let Message { message } = try_or_cry!(@warn -> message.ok_or("No message recieved")).into_inner();

    let time = time::OffsetDateTime::now_utc();

    let row = ErrorRow {
        err_id: Uuid::new_v4(),
        service,
        subservice,
        error_message: message,
        error_data_json: data,
        timestamp: time,
    };
    let mut inserter = try_or_cry!(@error -> clickhouse.client.insert("errs"));
    try_or_cry!(@error -> inserter.write(&row).await);
    try_or_cry!(@error -> inserter.end().await);

    ::log::info!(
        "Logged error from {:?} @ {:?}",
        service_path.as_ref(),
        time,
    );
    HttpResponse::build(StatusCode::OK).body("OK")
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().unwrap();
    env_logger::init();

    let app_data = clickhouse::Client::default()
        // .with_database("default")
        .with_url(std::env::var("CLICKHOUSE_URL").unwrap())
        .with_user(std::env::var("CLICKHOUSE_USER").unwrap())
        .with_password(std::env::var("CLICKHOUSE_PASSWORD").unwrap());
    let app_data = Data::new(ClickhouseState {
        client: Arc::new(app_data),
    });

    let port = std::env::var("PORT").unwrap().parse().unwrap();

    HttpServer::new(move || {
        App::new().app_data(app_data.clone()).service(log)
    })
        .bind(("127.0.0.1", port))?
        .run()
        .await
}

