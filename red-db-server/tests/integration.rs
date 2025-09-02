use std::{
    net::{SocketAddr, TcpListener},
    time::Duration,
};

use tempfile::tempdir;
use tokio::time::sleep;

use red_db_client::ClientBuilder;
use red_db_server::settings::Settings;

fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub async fn wait_for_port(port: u16, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();

    while start.elapsed().as_millis() < timeout_ms as u128 {
        if TcpListener::bind(format!("127.0.0.1:{port}")).is_err() {
            return true;
        }

        sleep(Duration::from_millis(10)).await;
    }

    false
}

async fn start_server() -> u16 {
    let _ = tracing_subscriber::fmt::try_init();

    let mut settings = Settings::default();
    let port = find_free_port();
    settings.port = port;
    settings.host = "127.0.0.1".to_string();

    let temp_dir = tempdir().expect("Failed to create temp dir");
    settings.aof_path = temp_dir
        .path()
        .join("test.rdb")
        .to_string_lossy()
        .to_string();

    tokio::spawn(async move {
        let result = red_db_server::run_server(settings).await;
        temp_dir.close().expect("Failed to close temp dir");

        result.expect("Failed to run server");
    });

    assert!(
        wait_for_port(port, 5 * 1000).await,
        "Server failed to start"
    );

    port
}

#[tokio::test]
async fn test_basic_connection() {
    let port = start_server().await;

    let client = ClientBuilder::new()
        .with_server_addr(SocketAddr::from(([127, 0, 0, 1], port)))
        .build()
        .await;

    assert!(client.is_ok(), "Failed to build client");

    let client = client.unwrap();

    client
        .create_space("test_space".to_string())
        .await
        .expect("Failed to create space");
    let space = client
        .space("test_space".to_string())
        .await
        .expect("Failed to get space");
    space
        .set_string("test_key", "test_value")
        .await
        .expect("Failed to set key");

    let result = space
        .get_string("test_key")
        .await
        .expect("Failed to get key");
    assert_eq!(result, Some("test_value".to_string()));
}
