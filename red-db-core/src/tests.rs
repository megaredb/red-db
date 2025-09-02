use tempfile::tempdir;

use crate::{
    db::Db,
    proto::{Command, Response},
};

#[tokio::test]
async fn test_basic_operations() {
    let temp_dir = tempdir().unwrap();
    let aof_path = temp_dir.path().join("test.aof");
    let db = Db::new(aof_path).await;
    let response = db
        .execute(Command::CreateSpace {
            space: "test".to_string(),
        })
        .await;
    assert!(matches!(response, Response::Ok));

    let response = db
        .execute(Command::Set {
            space: "test".to_string(),
            key: "key1".to_string(),
            value: b"value1".to_vec(),
        })
        .await;
    assert!(matches!(response, Response::Ok));

    let response = db
        .execute(Command::Get {
            space: "test".to_string(),
            key: "key1".to_string(),
        })
        .await;
    assert!(matches!(response, Response::Value(Some(v)) if v == b"value1"));
}

#[tokio::test]
async fn test_aof_recovery() {
    let temp_dir = tempdir().unwrap();
    let aof_path = temp_dir.path().join("recovery.aof");

    {
        let db = Db::new(aof_path.clone()).await;
        db.execute(Command::CreateSpace {
            space: "test".to_string(),
        })
        .await;
        db.execute(Command::Set {
            space: "test".to_string(),
            key: "key1".to_string(),
            value: b"persistent".to_vec(),
        })
        .await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let db = Db::new(aof_path).await;
    let response = db
        .execute(Command::Get {
            space: "test".to_string(),
            key: "key1".to_string(),
        })
        .await;

    assert!(matches!(response, Response::Value(Some(v)) if v == b"persistent"));
}
