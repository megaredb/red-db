use red_db_server::{run_server, settings::Settings};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    run_server(Settings::read()).await
}
