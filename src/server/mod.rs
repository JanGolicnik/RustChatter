use std::{collections::HashMap, sync::Arc, thread, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

use crate::{User, UserPermissionLevel};

pub enum EventType {
    SendMessage(String),
    SetUsername(String),
    SetModerator(String),
    SetOwner(String),
    Kick(String),
    Exit,
    Error(String),
}

pub struct Event {
    pub sender: Uuid,
    pub event_type: Option<EventType>,
}

pub struct Client {
    user: User,
    writer: OwnedWriteHalf,
}
pub struct Server {
    listener: TcpListener,
    events: Arc<Mutex<Vec<Event>>>,
    clients: Arc<Mutex<HashMap<Uuid, Client>>>,
}

impl Server {
    pub async fn new(ip: &str, port: &str) -> std::io::Result<Self> {
        let bind = format!("{ip}:{port}");
        let listener = TcpListener::bind(bind.as_str()).await?;

        Ok(Self {
            listener,
            events: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn run(&mut self) -> std::io::Result<()> {
        let clients = self.clients.clone();
        let events = self.events.clone();
        tokio::spawn(async move {
            let _ = Self::handle_events(clients, events).await;
        });

        loop {
            let (stream, _) = self.listener.accept().await?;

            let (reader, writer) = stream.into_split();

            let uuid = Uuid::new_v4();

            let mut clients = self.clients.lock().await;

            let permission_level = if clients.is_empty() {
                UserPermissionLevel::Owner
            } else {
                UserPermissionLevel::None
            };

            clients.insert(
                uuid,
                Client {
                    writer,
                    user: User {
                        username: None,
                        permission_level,
                    },
                },
            );

            let events = self.events.clone();
            tokio::spawn(async move {
                let _ = Self::read_client(reader, events, uuid).await;
            });
        }
    }

    pub async fn handle_events(
        clients: Arc<Mutex<HashMap<Uuid, Client>>>,
        events: Arc<Mutex<Vec<Event>>>,
    ) -> std::io::Result<()> {
        loop {
            thread::sleep(Duration::from_secs_f32(0.02));

            let mut events = events.lock().await;

            for event in events.iter() {
                let mut clients = clients.lock().await;
                handle_event(event, &event.sender, &mut clients).await?;
            }

            events.clear();
        }
    }

    pub async fn read_client(
        reader: OwnedReadHalf,
        events: Arc<Mutex<Vec<Event>>>,
        owner: Uuid,
    ) -> std::io::Result<()> {
        let reader = BufReader::new(reader);

        let mut lines = reader.lines();

        loop {
            let line = lines.next_line().await?;
            if let Some(line) = line {
                let event_type = parse_event_type(line.trim().to_string());
                let mut events = events.lock().await;
                events.push(Event {
                    sender: owner,
                    event_type,
                });
            }
        }
    }
}

fn parse_event_type(line: String) -> Option<EventType> {
    if line.is_empty() {
        return None;
    }

    let ret = match line.strip_prefix('/') {
        Some(args) => {
            let mut args = args.split(' ');
            if let Some(command) = args.next() {
                match command {
                    "username" => {
                        if let Some(name) = args.next() {
                            EventType::SetUsername(name.to_string())
                        } else {
                            EventType::Error("correct usage: /username [name]".to_string())
                        }
                    }
                    "setmod" => {
                        if let Some(target) = args.next() {
                            EventType::SetModerator(target.to_string())
                        } else {
                            EventType::Error("correct usage: /setmod [target]".to_string())
                        }
                    }
                    "setown" => {
                        if let Some(target) = args.next() {
                            EventType::SetUsername(target.to_string())
                        } else {
                            EventType::Error("correct usage: /setown [target]".to_string())
                        }
                    }
                    "kick" => {
                        if let Some(target) = args.next() {
                            EventType::SetUsername(target.to_string())
                        } else {
                            EventType::Error("correct usage: /username [username]".to_string())
                        }
                    }
                    "exit" => EventType::Exit,
                    _ => EventType::Error("unknown command".to_string()),
                }
            } else {
                EventType::Error("missing command ?".to_string())
            }
        }
        None => EventType::SendMessage(line),
    };

    Some(ret)
}

async fn handle_event(
    event: &Event,
    sender: &Uuid,
    clients: &mut MutexGuard<'_, HashMap<Uuid, Client>>,
) -> std::io::Result<()> {
    let event_type = match &event.event_type {
        None => return Ok(()),
        Some(val) => val,
    };

    let mut response: Option<String> = None;
    match event_type {
        EventType::SendMessage(content) => {
            let username: Option<String> = match clients.get(sender) {
                None => Some("Unknown".to_string()),
                Some(client) => client.user.username.clone(),
            };
            if let Some(username) = username {
                for (_, client) in clients.iter_mut() {
                    client
                        .writer
                        .write_all(format!("{username}: {content}\n").as_bytes())
                        .await?;
                }
            } else {
                response = Some(
                    "before sending messages set a username with /username [name]".to_string(),
                );
            }
        }
        EventType::SetUsername(username) => {
            let sender = match clients.get_mut(sender) {
                None => return Ok(()),
                Some(val) => val,
            };
            sender.user.username = Some(username.clone());
            response = Some("username set!".to_string());
        }
        EventType::SetModerator(target) => {
            let can_set = match clients.get(sender) {
                None => true,
                Some(client) => client.user.permission_level != UserPermissionLevel::None,
            };

            if can_set {
                response = Some("couldnt find user".to_string());
                for (_, client) in clients.iter_mut() {
                    if let Some(username) = &client.user.username {
                        if username == target {
                            client.user.permission_level = UserPermissionLevel::Mod;
                            response = Some("permissions set!".to_string());
                        }
                    }
                }
            } else {
                response = Some("insufficient permissions".to_string());
            }
        }
        EventType::SetOwner(target) => todo!(),
        EventType::Kick(target) => todo!(),
        EventType::Exit => todo!(),
        EventType::Error(content) => {
            response = Some(content.clone());
        }
    }

    if let Some(response) = response {
        let sender = match clients.get_mut(sender) {
            None => return Ok(()),
            Some(val) => val,
        };
        sender
            .writer
            .write_all(format!("{response}\n").as_bytes())
            .await?;
    }

    Ok(())
    // let data = format!("{}: {}\n", name_tag, message.content);

    // for (_uuid, client) in clients.iter_mut() {
    //     client.writer.write_all(data.as_bytes()).await?;
    // }
}
