use super::*;
use std::panic;
use tempfile::tempdir;

async fn create_test_client() -> (Client, tempfile::TempDir) {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("test_db.rdb");

    let client = ClientBuilder::new()
        .with_aof_path(db_path)
        .with_max_pool_size(4)
        .build()
        .await
        .expect("Failed to build client");

    (client, dir)
}

#[tokio::test]
async fn test_builder_builds_successfully_with_aof() {
    let (_client, _dir) = create_test_client().await;
}

#[test]
fn test_builder_panics_on_conflicting_config() {
    let result = panic::catch_unwind(|| {
        ClientBuilder::new()
            .with_server_addr("127.0.0.1:8080".parse::<SocketAddr>().unwrap())
            .with_aof_path(PathBuf::from("/tmp/db"));
    });
    assert!(
        result.is_err(),
        "Should panic when setting aof_path after server_addr"
    );

    let result = panic::catch_unwind(|| {
        ClientBuilder::new()
            .with_aof_path(PathBuf::from("/tmp/db"))
            .with_server_addr("127.0.0.1:8080".parse::<SocketAddr>().unwrap());
    });
    assert!(
        result.is_err(),
        "Should panic when setting server_addr after aof_path"
    );
}

#[tokio::test]
async fn test_builder_panics_without_config() {
    let result = ClientBuilder::new().build().await;
    assert!(
        result.is_err(),
        "Should panic when neither server_addr nor aof_path is set"
    );
}

#[tokio::test]
async fn test_create_and_check_space_existence() {
    let (client, _dir) = create_test_client().await;
    let space_name = "my_space".to_string();

    let exists = client.is_space_exists(space_name.clone()).await.unwrap();
    assert!(!exists, "Space should not exist initially");

    client.create_space(space_name.clone()).await.unwrap();

    let exists = client.is_space_exists(space_name.clone()).await.unwrap();
    assert!(exists, "Space should exist after creation");
}

#[tokio::test]
async fn test_delete_space() {
    let (client, _dir) = create_test_client().await;
    let space_name = "space_to_delete".to_string();

    client.create_space(space_name.clone()).await.unwrap();
    let exists = client.is_space_exists(space_name.clone()).await.unwrap();
    assert!(exists, "Space must exist before deletion");

    client.delete_space(space_name.clone()).await.unwrap();

    let exists = client.is_space_exists(space_name.clone()).await.unwrap();
    assert!(!exists, "Space should not exist after deletion");
}

#[tokio::test]
async fn test_get_space_client() {
    let (client, _dir) = create_test_client().await;
    let space_name = "existing_space".to_string();
    let non_existing_space = "non_existing_space".to_string();

    let result = client.space(non_existing_space).await;
    assert!(result.is_err(), "Should fail for a non-existing space");
    if let Err(ClientError::Server(server_error)) = result {
        assert!(matches!(
            server_error,
            red_db_core::error::ServerError::SpaceNotFound(_)
        ));
    } else {
        panic!("Expected a ServerError::SpaceNotFound");
    }

    client.create_space(space_name.clone()).await.unwrap();
    let result = client.space(space_name).await;
    assert!(result.is_ok(), "Should succeed for an existing space");
}

#[tokio::test]
async fn test_set_get_delete_flow() {
    let (client, _dir) = create_test_client().await;
    let space_name = "test_space".to_string();
    let key = "my_key";
    let value = vec![1, 2, 3, 4, 5];

    client.create_space(space_name.clone()).await.unwrap();
    let space_client = client.space(space_name).await.unwrap();

    let result = space_client.get(key).await.unwrap();
    assert_eq!(
        result, None,
        "Getting a non-existent key should return None"
    );

    space_client.set(key, value.clone()).await.unwrap();

    let result = space_client.get(key).await.unwrap();
    assert_eq!(
        result,
        Some(value),
        "Getting an existing key should return its value"
    );

    space_client.delete(key).await.unwrap();

    let result = space_client.get(key).await.unwrap();
    assert_eq!(result, None, "Getting a deleted key should return None");
}

#[tokio::test]
async fn test_string_helpers() {
    let (client, _dir) = create_test_client().await;
    let space_name = "string_space".to_string();
    let key = "greeting";
    let value = "Hello, World!";

    client.create_space(space_name.clone()).await.unwrap();
    let space_client = client.space(space_name).await.unwrap();

    space_client.set_string(key, value).await.unwrap();

    let result = space_client.get_string(key).await.unwrap();
    assert_eq!(
        result,
        Some(value.to_string()),
        "get_string should retrieve the correct string"
    );
}

#[tokio::test]
async fn test_list_keys() {
    let (client, _dir) = create_test_client().await;
    let space_name = "list_keys_space".to_string();

    client.create_space(space_name.clone()).await.unwrap();
    let space_client = client.space(space_name).await.unwrap();

    let keys = space_client.list_keys().await.unwrap();
    assert!(keys.is_empty(), "Key list should be empty initially");

    let key1 = "key1";
    let key2 = "key2";
    space_client.set(key1, vec![1]).await.unwrap();
    space_client.set(key2, vec![2]).await.unwrap();

    let mut keys = space_client.list_keys().await.unwrap();
    keys.sort();
    assert_eq!(keys.len(), 2, "Should have 2 keys");
    assert_eq!(keys, vec![key1.to_string(), key2.to_string()]);

    space_client.delete(key1).await.unwrap();
    let keys = space_client.list_keys().await.unwrap();
    assert_eq!(keys.len(), 1, "Should have 1 key after deletion");
    assert_eq!(keys[0], key2.to_string());
}
