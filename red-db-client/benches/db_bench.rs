use std::net::SocketAddr;

use criterion::{Criterion, criterion_group, criterion_main};
use red_db_client::{Client, ClientBuilder};
use tokio::runtime::Runtime;

async fn setup_client() -> Client {
    let client = ClientBuilder::new()
        .with_server_addr(SocketAddr::from(([127, 0, 0, 1], 25500)))
        .with_max_pool_size(16)
        .build()
        .await
        .expect("Failed to build client");

    let space_name = "benchmark_space".to_string();

    if !client.is_space_exists(space_name.clone()).await.unwrap() {
        client
            .create_space(space_name)
            .await
            .expect("Failed to create space");
    }

    client
}

fn bench_set(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let client = runtime.block_on(setup_client());
    let space_client = runtime
        .block_on(client.space("benchmark_space".to_string()))
        .unwrap();

    let key = "my_key";
    let value = "some value data that we want to write";

    c.bench_function("set_string", |b| {
        b.to_async(&runtime).iter(|| async {
            space_client.set_string(key, value).await.unwrap();
        });
    });
}

fn bench_get(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let client = runtime.block_on(setup_client());
    let space_client = runtime
        .block_on(client.space("benchmark_space".to_string()))
        .unwrap();

    let key = "my_key_for_get";
    let value = "some value for get";

    // Записываем значение перед тем, как его читать
    runtime.block_on(async {
        space_client.set_string(key, value).await.unwrap();
    });

    c.bench_function("get_string", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = space_client.get_string(key).await.unwrap();
            assert!(result.is_some());
        });
    });
}

criterion_group!(benches, bench_set, bench_get);
criterion_main!(benches);
