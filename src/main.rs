use anyhow::Result;
use rmcp::transport::io::stdio;
use rmcp::ServiceExt;
use tracing::info;

mod db;
mod tools;

use tools::FeaturesMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = dataxlr8_mcp_core::Config::from_env("dataxlr8-features-mcp")
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    dataxlr8_mcp_core::logging::init(&config.log_level);

    info!(
        server = config.server_name,
        "Starting DataXLR8 Features MCP server"
    );

    let database = dataxlr8_mcp_core::Database::connect(&config.database_url)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Run schema setup
    db::setup_schema(database.pool()).await?;

    let server = FeaturesMcpServer::new(database);

    let transport = stdio();
    let service = server.serve(transport).await?;

    info!("Features MCP server connected via stdio");
    service.waiting().await?;

    Ok(())
}
