use std::env::args;
use std::io::{stdout, Write};

use chat::client::{Client, ClientReader, ClientWriter};
use chat::server::Server;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind};
use crossterm::style::Print;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, queue,
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

    let client_writer = client.get_writer();
    let process_input = tokio::spawn(async move {
        process_client_input(&client_writer).await.unwrap();
    });

    let client_reader = client.get_reader();
    let read = tokio::spawn(async move {
        read_client(&client_reader).await.unwrap();
    });

    process_input.await?;
    read.await?;

    execute!(stdout, DisableMouseCapture)?;
    disable_raw_mode()?;

    Ok(())
}

async fn process_client_input(writer: &ClientWriter) -> std::io::Result<()> {
    let mut current_input = String::new();

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                if kind != KeyEventKind::Press {
                    continue;
                }

                match code {
                    KeyCode::Char(c) => {
                        current_input.push(c);
                    }
                    KeyCode::Enter => {
                        writer.write(format!("{current_input}\n")).await?;
                        current_input.clear();
                    }
                    KeyCode::Esc => break,
                    KeyCode::Backspace => {
                        current_input.pop();
                    }
                    _ => {}
                }

                let height = crossterm::terminal::size()?.1 - 1;
                execute!(
                    stdout(),
                    crossterm::cursor::MoveTo(0, height),
                    crossterm::terminal::Clear(ClearType::CurrentLine),
                    Print(current_input.clone())
                )?;
            }
        }
    }
    Ok(())
}

async fn read_client(reader: &ClientReader) -> std::io::Result<()> {
    let mut lines: Vec<String> = Vec::new();

    while let Some(line) = reader.read().await? {
        lines.push(line.trim().to_string());
        let height = (crossterm::terminal::size()?.1 - 2) as usize;

        let mut stdout = stdout();

        queue!(
            stdout,
            crossterm::cursor::MoveTo(0, height as u16),
            crossterm::terminal::Clear(ClearType::FromCursorUp),
            crossterm::terminal::Clear(ClearType::CurrentLine),
        )?;

        let mut line_index = lines.len() as i32;
        for i in 0..height {
            let cursor_y = height - i;
            if line_index >= 0 {
                line_index -= 1;
            }
            let line = lines.get(line_index as usize);
            queue!(
                stdout,
                crossterm::cursor::MoveTo(0, cursor_y as u16),
                Print(if let Some(line) = line { line } else { "" })
            )?;
        }
        queue!(
            stdout,
            crossterm::cursor::MoveTo(0, crossterm::terminal::size()?.1)
        )?;

        stdout.flush()?;
    }

    Ok(())
}
