use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
};
pub struct Server {
    listener: TcpListener,
    pub lines: Arc<Mutex<Vec<String>>>,
}

impl Server {
    pub async fn new(ip: &str, port: &str) -> std::io::Result<Self> {
        let bind = format!("{ip}:{port}");
        let listener = TcpListener::bind(bind.as_str()).await?;

        Ok(Self {
            listener,
            lines: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let lines_clone = self.lines.clone();

            let (reader, writer) = stream.into_split();

            let read = tokio::spawn(async move {
                let _ = Self::read_client(reader, lines_clone).await;
            });
            let write = tokio::spawn(async move {
                let _ = Self::write_client(writer).await;
            });

            read.await.unwrap();
            write.await.unwrap();
        }
    }

    pub async fn write_client(mut writer: OwnedWriteHalf) -> std::io::Result<()> {
        loop {
            writer.write_all(b"WOW\n").await?;
            println!("sent");
            thread::sleep(Duration::from_secs_f32(0.5));
        }
    }

    pub async fn read_client(
        reader: OwnedReadHalf,
        server_lines: Arc<Mutex<Vec<String>>>,
    ) -> std::io::Result<()> {
        let reader = BufReader::new(reader);

        let mut lines = reader.lines();

        loop {
            let line = lines.next_line().await?;
            if let Some(line) = line {
                let mut lock = server_lines.lock().unwrap();
                lock.push(line);
            }
        }
    }
}
