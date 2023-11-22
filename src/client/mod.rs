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
    pub writer: Arc<Mutex<OwnedWriteHalf>>,
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

    pub async fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        let mut lock = self.writer.lock().await;
        lock.write_all(data).await?;
        Ok(())
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
