# red-db 🦀

red-db is a simple, fast, and persistent key-value database written in pure Rust. This project was created as a learning exercise to explore database development, asynchronous programming with `tokio`, and API design in Rust.

It's designed for versatility, offering two primary modes of operation:

  * **Standalone Server**: Run `red-db` as a dedicated TCP server that your applications can connect to.
  * **Embedded Mode**: Embed `red-db` directly into your application to work with a local database file (`.rdb`), eliminating the need for a separate server process.

-----

## Features

  * **Key-Value Store**: Simple API for `set`, `get`, and `delete` operations.
  * **Namespaces ("Spaces")**: Organize your data into isolated collections called "spaces".
  * **Dual Operation Modes**: Use as a client-server database or as an embedded library.
  * **Persistent Storage**: Uses an **Append-Only File (AOF)** strategy to ensure data durability.
  * **Asynchronous API**: Built with `tokio` for non-blocking I/O.
  * **Connection Pooling**: The client comes with a built-in `deadpool` connection pool for efficient server communication.
  * **Simple Binary Protocol**: Uses `bincode` for fast and efficient data serialization.

-----

## Project Structure

The project is a Rust workspace divided into three main crates:

  * `red-db-server`: The standalone database server. It listens for TCP connections, processes commands, and manages the database file.
  * `red-db-client`: The client library for your applications. It provides a clean API to interact with the database in either standalone or embedded mode.
  * `red-db-core`: The shared logic between the server and client, including the command/response protocol and the core database engine.

-----

## Getting Started

To use `red-db` in your project, add the client library to your `Cargo.toml`:

```toml
[dependencies]
red-db-client = { path = "path/to/red-db/red-db-client" }
# Or if published: red-db-client = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

### Usage Example 1: Embedded Mode

In this mode, the client interacts directly with a database file. This is perfect for applications that need a simple, local database without network overhead.

```rust
use red_db_client::{ClientBuilder, ClientError};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    // 1. Build a client that works directly with a file.
    let client = ClientBuilder::new()
        .with_aof_path(PathBuf::from("my_app.rdb"))
        .build()
        .await?;

    // 2. Create a "space" to organize your data.
    client.create_space("users".to_string()).await?;

    // 3. Get a handle to the space.
    let users_space = client.space("users".to_string()).await?;

    // 4. Set a key-value pair.
    users_space.set_string("user:101", "Alice").await?;
    println!("Saved user 'Alice'");

    // 5. Get the value back.
    if let Some(name) = users_space.get_string("user:101").await? {
        println!("Retrieved user: {}", name); // -> Retrieved user: Alice
    }

    // 6. List all keys in the space.
    let keys = users_space.list_keys().await?;
    println!("Keys in 'users' space: {:?}", keys); // -> Keys in 'users' space: ["user:101"]

    Ok(())
}
```

### Usage Example 2: Client-Server Mode

For this mode, you first need to run the `red-db-server`, and then your application can connect to it over the network.

**Step 1: Run the Server**

Clone the repository and run the server from your terminal. It will use the settings in `config.toml` to start listening for connections.

```bash
# From the root of the red-db project
cargo run --package red-db-server
```

**Step 2: Write the Client Code**

Now, in your separate application, use the `ClientBuilder` to connect to the server's address.

```rust
use red_db_client::{ClientBuilder, ClientError};

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    // 1. Build a client that connects to the server.
    let client = ClientBuilder::new()
        .with_server_addr("127.0.0.1:25500")
        .build()
        .await?;

    // 2. The rest of the API is identical to the embedded mode!
    client.create_space("products".to_string()).await?;
    let products_space = client.space("products".to_string()).await?;

    // Set a key with raw bytes
    products_space.set("product:456", vec![10, 20, 30]).await?;
    println!("Saved product data");

    // Get the raw bytes back
    if let Some(data) = products_space.get("product:456").await? {
        println!("Retrieved product data: {:?}", data); // -> Retrieved product data: [10, 20, 30]
    }

    Ok(())
}
```

-----

## Configuration

The `red-db-server` is configured using a `config.toml` file in the project's root directory.

```toml
# The host address to bind the server to.
host = "127.0.0.1"

# The port to listen on for incoming TCP connections.
port = 25500

# The path to the Append-Only File (AOF) for data persistence.
aof_path = "aof.rdb"
```

-----

## License

This project is licensed under the **MIT License**. See the `LICENSE` file for details.
