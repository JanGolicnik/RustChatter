use std::sync::Arc;
use std::{env::args, thread, time::Duration};

use chat::client::Client;
use chat::server::Server;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;

const TCP_PORT: &str = "8100";

enum ConnectionMode {
    Server,
    Client,
}

#[tokio::main]
async fn main() {
    let mut args = args();

    let mut ip = "127.0.0.1".to_string();

    let mut connection_mode: ConnectionMode = ConnectionMode::Client;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "/s" => {
                connection_mode = ConnectionMode::Server;
            }
            "-ip" => {
                if let Some(ip_arg) = args.next() {
                    ip = ip_arg;
                }
            }
            _ => {}
        }
    }

    match connection_mode {
        ConnectionMode::Client => match run_client(ip).await {
            Ok(_) => {}
            Err(err) => println!("{err}"),
        },
        ConnectionMode::Server => match run_server(ip).await {
            Ok(_) => {}
            Err(err) => println!("{err}"),
        },
    }
}

async fn run_server(ip: String) -> std::io::Result<()> {
    let mut server = Server::new(&ip, TCP_PORT).await?;

    server.run().await?;
    Ok(())
}
async fn run_client(ip: String) -> std::io::Result<()> {
    let mut client = Client::new(&ip, TCP_PORT).await?;

    let write_half = client.writer.clone();
    tokio::spawn(async move {
        loop {
            let _ = process_client_input(write_half.clone()).await;
            thread::sleep(Duration::from_secs_f32(0.5))
        }
    });

    client.run().await;

    Ok(())
}

async fn process_client_input(
    writer: Arc<tokio::sync::Mutex<OwnedWriteHalf>>,
) -> std::io::Result<()> {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("error: unable to read user input");
    let mut writer = writer.lock().await;
    writer.write_all(input.as_bytes()).await.unwrap();
    Ok(())
}
