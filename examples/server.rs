use ghost_sync::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::builder()
        .bind("0.0.0.0:7777")
        .max_clients(8)
        .max_payload(64 * 1024)
        .channel_capacity(16)
        .build();

    server.create_room("chatroom")?;

    let handle = server.run().await?;

    println!("Press Ctrl+C to stop...");
    tokio::signal::ctrl_c().await?;
    handle.shutdown().await;
    println!("Shutting down.");

    Ok(())
}
