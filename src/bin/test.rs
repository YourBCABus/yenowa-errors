use serde_json::json;
use yenowa_errors::{ init_env, report };

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenvy::dotenv().unwrap();

    init_env("test", "test").unwrap();

    report("Test message", &json!({ "test": "data" }))
        .await
        .expect("Failed to report error");
}
