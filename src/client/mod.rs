use std::sync::Arc;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

pub struct Client {
    reader: Arc<Mutex<BufReader<OwnedReadHalf>>>,
    writer: Arc<Mutex<OwnedWriteHalf>>,
}
pub struct ClientWriter {
    writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl Client {
    pub async fn new(ip: &str, port: &str) -> std::io::Result<Self> {
        let bind = format!("{ip}:{port}");
        let stream = TcpStream::connect(bind.as_str())
            .await
            .unwrap_or_else(|_| panic!("Failed to connect to {bind}"));
        let (reader, writer) = stream.into_split();
        let reader = BufReader::new(reader);
        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }

    pub fn get_writer(&self) -> ClientWriter {
        ClientWriter {
            writer: self.writer.clone(),
        }
    }

    pub async fn run(&mut self) {
        let reader_clone = self.reader.clone();
        let f = tokio::spawn(async move {
            Self::listen_for_messages(reader_clone).await.unwrap();
        });
        f.await.unwrap();
    }

    async fn listen_for_messages(
        reader: Arc<Mutex<BufReader<OwnedReadHalf>>>,
    ) -> std::io::Result<()> {
        let mut reader = reader.lock().await;
        let mut line: String = "".to_string();
        loop {
            match reader.read_line(&mut line).await {
                Ok(0) => return Ok(()),
                Ok(_) => println!("{line}"),
                Err(e) => eprintln!("{e}"),
            };
            line.clear();
        }
    }
}

impl ClientWriter {
    pub async fn write(&self, text: String) -> std::io::Result<()> {
        let mut lock = self.writer.lock().await;
        lock.write_all(text.as_bytes()).await?;
        Ok(())
    }
}
