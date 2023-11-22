use std::{env::args, thread, time::Duration};

use chat::client::{Client, ClientReader, ClientWriter};
use chat::server::Server;

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
    let client = Client::new(&ip, TCP_PORT).await?;

    let client_writer = client.get_writer();
    let process_input = tokio::spawn(async move {
        process_client_input(&client_writer).await.unwrap();
    });

    let client_reader = client.get_reader();
    let refresh = tokio::spawn(async move {
        refresh_client(&client_reader).await.unwrap();
    });

    process_input.await?;
    refresh.await?;

    Ok(())
}

async fn process_client_input(writer: &ClientWriter) -> std::io::Result<()> {
    loop {
        thread::sleep(Duration::from_secs_f32(0.1));
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("error: unable to read user input");

        writer.write(input).await?;
    }
}

async fn refresh_client(reader: &ClientReader) -> std::io::Result<()> {
    println!("reading");
    while let Some(line) = reader.read().await? {
        println!("{line}");
    }

    Ok(())
}
