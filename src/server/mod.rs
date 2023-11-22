use std::{collections::HashMap, sync::Arc, thread, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
    sync::Mutex,
};
use uuid::Uuid;

pub struct Message {
    pub sender: Uuid,
    pub content: String,
}

pub struct Client {
    name: String,
    writer: OwnedWriteHalf,
}
pub struct Server {
    listener: TcpListener,
    messages: Arc<Mutex<Vec<Message>>>,
    clients: Arc<Mutex<HashMap<Uuid, Client>>>,
}

impl Server {
    pub async fn new(ip: &str, port: &str) -> std::io::Result<Self> {
        let bind = format!("{ip}:{port}");
        let listener = TcpListener::bind(bind.as_str()).await?;

        Ok(Self {
            listener,
            messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        let clients = self.clients.clone();
        let messages = self.messages.clone();
        tokio::spawn(async move {
            let _ = Self::broadcast_messages(clients, messages).await;
        });

        let mut user_counter: u32 = 1;

        loop {
            let (stream, _) = self.listener.accept().await?;

            let (reader, writer) = stream.into_split();

            let uuid = Uuid::new_v4();
            let name = format!("User {}", user_counter);
            user_counter += 1;

            let mut clients = self.clients.lock().await;
            clients.insert(uuid, Client { writer, name });

            let messages = self.messages.clone();
            tokio::spawn(async move {
                let _ = Self::read_client(reader, messages, uuid).await;
            });
        }
    }

    pub async fn broadcast_messages(
        clients: Arc<Mutex<HashMap<Uuid, Client>>>,
        messages: Arc<Mutex<Vec<Message>>>,
    ) -> std::io::Result<()> {
        loop {
            thread::sleep(Duration::from_secs_f32(0.02));

            let mut messages = messages.lock().await;

            for message in messages.iter() {
                let mut clients = clients.lock().await;

                let sender = clients.get(&message.sender);
                let name_tag = match sender {
                    Some(v) => v.name.clone(),
                    None => "Unknown".to_string(),
                };

                let data = format!("{}: {}\n", name_tag, message.content);

                for (_uuid, client) in clients.iter_mut() {
                    client.writer.write_all(data.as_bytes()).await?;
                }
            }

            messages.clear();
        }
    }

    pub async fn read_client(
        reader: OwnedReadHalf,
        messages: Arc<Mutex<Vec<Message>>>,
        owner: Uuid,
    ) -> std::io::Result<()> {
        let reader = BufReader::new(reader);

        let mut lines = reader.lines();

        loop {
            let line = lines.next_line().await?;
            if let Some(line) = line {
                let mut messages = messages.lock().await;
                messages.push(Message {
                    sender: owner,
                    content: line,
                });
            }
        }
    }
}
