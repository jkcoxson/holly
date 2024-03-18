// Jackson Coxson

use std::{collections::HashMap, sync::Arc};

use chat::ChatMessage;
use log::{error, info};
use thirtyfour::error::WebDriverResult;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

mod browser;
mod chat;
mod config;

async fn entry(clear_cookies: bool) -> WebDriverResult<()> {
    env_logger::init();
    info!("Logger initialized");
    
    let config = config::Config::load();
    let client = browser::Browser::new(&config).await.unwrap();

    if !clear_cookies {
        client.load_cookies().await.unwrap();
    }

    if !client.is_logged_in().await
        && client
            .login(&config.fb_username, &config.fb_password)
            .await
            .is_err()
    {
        client.delete_cookies().await.unwrap();
        client
            .login(&config.fb_username, &config.fb_password)
            .await
            .unwrap();
        client.dump_cookies().await.unwrap();
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
                                    if let Ok(buf) = String::from_utf8(buf[0..x].to_vec()) {
                                        if x == 0 {
                                            break;
                                        }
                                        // Split the buf into JSON packets
                                        // As we've learned, sometimes nagle's algo will squish them
                                        // together into one packet, so we need to split them up
                                        let packets = buf.split("}{")
                                            .map(|s| {
                                                let s = if s.ends_with('}') {
                                                    s.to_string()
                                                } else {
                                                    format!("{s}}}")
                                                };
                                                if s.starts_with('{') {
                                                    s.to_string()
                                                } else {
                                                    format!("{{{s}")
                                                }
                                            })
                                            .collect::<Vec<_>>();

                                        for packet in packets {
                                            if let Ok(msg) = serde_json::from_str::<ChatMessage>(&packet) {
                                                tx.send(msg).await.unwrap();
                                            } else {
                                                println!("Failed to parse msg: {:?}", buf);
                                            }
                                        }
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

    let mut error_count: u8 = 0;

    println!("Startup complete");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Decline calls
        if let Err(e) = client.decline_call().await {
            println!("Unable to decline call: {:?}", e);
        }

        // See if the current chat has different messages than before
        let current_message = match client.get_messages().await {
            Ok(c) => c
                .last()
                .unwrap_or(&ChatMessage {
                    sender: "".to_string(),
                    content: "".to_string(),
                    chat_id: "".to_string(),
                })
                .to_owned(),
            Err(e) => {
                println!("Unable to get messages: {:?}", e);
                error_count += 1;
                if error_count > 10 {
                    return Err(e);
                }
                continue;
            }
        };

        let current_chat = match client.get_current_chat().await {
            Ok(c) => c,
            Err(e) => {
                println!("Unable to get current chat: {:?}", e);
                error_count += 1;
                if error_count > 10 {
                    return Err(e);
                }
                continue;
            }
        };

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
            match msg.sender.as_str() {
                "<screenshot>" => {
                    if let Err(e) = client.screenshot_log().await {
                        error!("Unable to take screenshot!");
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                    }
                    continue;
                }
                "<html>" => {
                    if let Err(e) = client.html_log().await {
                        error!("Unable to take html log!");
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                    }
                    continue;
                }
                "<restart>" => return Ok(()),
                "<refresh>" => {
                    client.refresh().await?;
                    continue;
                }
                _ => {
                    println!("Sending message: {:?}", msg);
                    if let Err(e) = client.go_to_chat(&msg.chat_id).await {
                        println!("Unable to go to chat for send: {:?}", e);
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                        continue;
                    }
                    if let Err(e) = client.send_message(&msg.content).await {
                        println!("Unable to send message: {:?}", e);
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                        continue;
                    }
                    continue;
                }
            }
        }

        // Check for unread messages
        let mut chats = match client.get_chats().await {
            Ok(chats) => chats,
            Err(e) => {
                println!("Unable to get chats: {:?}", e);
                error_count += 1;
                if error_count > 10 {
                    return Err(e);
                }
                continue;
            }
        };
        chats.retain(|chat| chat.unread);
        if !chats.is_empty() {
            if chats[0].click().await.is_err() {
                if let Err(e) = client.refresh().await {
                    println!("Unable to refresh, aborting Holly!");
                    error_count += 1;
                    if error_count > 10 {
                        return Err(e);
                    }
                    return Err(e);
                }
                continue;
            }

            // If this is the first time we've accessed this, fill with nonsense
            last_messages
                .entry(chats[0].id.clone())
                .or_insert_with(|| ChatMessage {
                    content: "nonsense".to_string(),
                    chat_id: chats[0].id.clone(),
                    sender: "asdf".to_string(),
                });
            continue;
        }

        // Until next time *rides motorcycle away*
    }
}

#[tokio::main]
async fn main() {
    println!("Starting Holly core...");

    let mut last_error = std::time::Instant::now();
    let mut clear_cookies = false;

    loop {
        if let Err(e) = entry(clear_cookies).await {
            println!("Holly crashed with {:?}", e);
            if last_error.elapsed().as_secs() > 60 {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                println!("Restarting Holly...");
                last_error = std::time::Instant::now();
                clear_cookies = false;
            } else if clear_cookies {
                panic!("Holly has run into an unrecoverable state!")
            } else {
                clear_cookies = true;
            }
        }
    }
}
