use std::env::args;
use std::io::stdout;
use std::sync::Arc;

use chat::client::{Client, ClientReader, ClientWriter};
use chat::server::Server;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::Mutex;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

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
    enable_raw_mode()?;

    let client = Client::new(&ip, TCP_PORT).await?;

    let mut stdout = stdout();
    execute!(stdout, EnableMouseCapture)?;
    execute!(stdout, Clear(ClearType::Purge))?;

    let current_user_input = Arc::new(Mutex::new(String::new()));
    let current_user_input_clone = current_user_input.clone();
    let client_writer = client.get_writer();
    let process_input = tokio::spawn(async move {
        process_client_input(&client_writer, current_user_input_clone)
            .await
            .unwrap();
    });

    let client_reader = client.get_reader();
    let refresh = tokio::spawn(async move {
        refresh_client(&client_reader, current_user_input)
            .await
            .unwrap();
    });

    process_input.await?;
    refresh.await?;

    execute!(stdout, DisableMouseCapture)?;
    disable_raw_mode()?;

    Ok(())
}

async fn process_client_input(
    writer: &ClientWriter,
    current_user_input: Arc<Mutex<String>>,
) -> std::io::Result<()> {
    // loop {
    //     thread::sleep(Duration::from_secs_f32(0.1));
    //     let mut input = String::new();
    //     std::io::stdin()
    //         .read_line(&mut input)
    //         .expect("error: unable to read user input");

    //     writer.write(input).await?;
    // }

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                if kind != KeyEventKind::Press {
                    continue;
                }

                match code {
                    KeyCode::Char(c) => {
                        let mut current_user_input = current_user_input.lock().await;
                        current_user_input.push(c);
                    }
                    KeyCode::Enter => {
                        let mut current_user_input = current_user_input.lock().await;
                        writer.write(format!("{current_user_input}\n")).await?;
                        current_user_input.clear();
                    }
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn refresh_client(
    reader: &ClientReader,
    current_user_input: Arc<Mutex<String>>,
) -> std::io::Result<()> {
    let mut received_lines: Vec<String> = Vec::new();
    let mut stdout = stdout();
    while let Some(line) = reader.read().await? {
        received_lines.push(line);

        execute!(stdout, Clear(ClearType::Purge))?;
        for line in received_lines.iter() {
            println!("{line}");
        }

        execute!(
            stdout,
            crossterm::cursor::MoveTo(0, crossterm::terminal::size()?.1 - 1)
        )?;

        let current_user_input = current_user_input.lock().await;
        println!("{current_user_input}");
    }

    Ok(())
}
