// Jackson Coxson

use std::{collections::HashMap, sync::Arc};

use chat::ChatMessage;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

mod browser;
mod chat;
mod config;

#[tokio::main]
async fn main() {
    let config = config::Config::load();
    let client = browser::Browser::new(&config).await.unwrap();
    client.load_cookies().await.unwrap();
    if !client.is_logged_in().await {
        if client
            .login(&config.fb_username, &config.fb_password)
            .await.is_err() {
                client.delete_cookies().await.unwrap();
                client.login(
                    &config.fb_username,
                    &config.fb_password,
                ).await.unwrap();
                client.dump_cookies().await.unwrap();
            }
    }
    client.dump_cookies().await.unwrap();

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.tcp.host, config.tcp.port))
            .await
            .unwrap();

    let senders = Arc::new(Mutex::new(Vec::new()));
    let tcp_senders = senders.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatMessage>(100);

    tokio::spawn(async move {
        loop {
            if let Ok((mut stream, addr)) = listener.accept().await {
                println!("Accepted connection from {:?}", addr);

                let (local_tx, mut local_rx) = tokio::sync::mpsc::channel::<ChatMessage>(100);
                let tx = tx.clone();
                tcp_senders.lock().await.push(local_tx);

                tokio::spawn(async move {
                    loop {
                        let mut buf = [0; 4096];
                        tokio::select! {
                            msg = local_rx.recv() => {
                                let msg = serde_json::to_string(&msg).unwrap();
                                if stream.write(msg.as_bytes()).await.is_err() {
                                    break;
                                }
                            }
                            x = stream.read(&mut buf) => {
                                if let Ok(x) = x {
                                    let buf = &buf[0..x];
                                    if let Ok(msg) = serde_json::from_slice::<ChatMessage>(&buf) {
                                        println!("Accepted msg: {:?}", msg);
                                        tx.send(msg).await.unwrap();
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                });
            }
        }
    });

    let mut last_messages = HashMap::new();
    let current_chat = client.get_current_chat().await.unwrap();
    client.screenshot_log().await.unwrap();
    last_messages.insert(
        current_chat,
        client
            .get_messages()
            .await
            .unwrap()
            .last()
            .unwrap_or(&ChatMessage {
                sender: "".to_string(),
                content: "".to_string(),
                chat_id: "".to_string(),
            })
            .to_owned(),
    );

    println!("Startup complete");
    loop {
        // See if the current chat has different messages than before
        let current_message = client
            .get_messages()
            .await
            .unwrap()
            .last()
            .unwrap_or(&ChatMessage {
                sender: "".to_string(),
                content: "".to_string(),
                chat_id: "".to_string(),
            }).to_owned();
        let current_chat = client.get_current_chat().await.unwrap();

        let last_message = last_messages.insert(current_chat.clone(), current_message.clone());

        if let Some(last_message) = last_message {
            if last_message != current_message {
                println!("{}: {}", current_chat, current_message.content);
                // Send to all clients
                let blocking_message = current_message.clone();
                let blocking_senders = senders.clone();
                tokio::task::spawn_blocking(move || {
                    blocking_senders
                        .blocking_lock()
                        .retain(|sender| sender.blocking_send(blocking_message.clone()).is_ok());
                });
            }
        }

        // Possibly send a message
        if let Ok(msg) = rx.try_recv() {
            println!("Sending message: {:?}", msg);
            client.go_to_chat(&msg.chat_id).await.unwrap();
            client.send_message(&msg.content).await.unwrap();
            continue;
        }

        // Check for unread messages
        let mut chats = client.get_chats().await.unwrap();
        chats.retain(|chat| chat.unread);
        if !chats.is_empty() {
            if chats[0].click().await.is_err() {
                client.refresh().await.unwrap();
                continue;
            }

            // If this is the first time we've accessed this, fill with nonsense
            if !last_messages.contains_key(&chats[0].id.clone()) {
                last_messages.insert(
                    chats[0].id.clone(),
                    ChatMessage {
                        content: "nonsense".to_string(),
                        chat_id: chats[0].id.clone(),
                        sender: "asdf".to_string(),
                    },
                );
            }
            continue;
        }

        // Until next time *rides motorcycle away*
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
