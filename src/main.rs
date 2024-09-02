// Jackson Coxson

use std::sync::Arc;

use chat::ChatMessage;
use log::{debug, error, info, warn};
use thirtyfour::error::WebDriverResult;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use crate::cache::Cache;

mod browser;
mod cache;
mod chat;
mod config;

async fn entry(clear_cookies: bool) -> WebDriverResult<()> {
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
        warn!("Cookies are invalid, logging in again");
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
                info!("Accepted connection from {:?}", addr);

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
                                if stream.flush().await.is_err() {
                                    warn!("Unable to flush message to client");
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
                                            if let Ok(mut msg) = serde_json::from_str::<ChatMessage>(&packet) {
                                                msg.clean();
                                                tx.send(msg).await.unwrap();
                                            } else {
                                                warn!("Failed to parse msg: {:?}", buf);
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

    let mut cache = Cache::new();
    let current_chat = client.get_current_chat().await.unwrap();
    cache
        .check(&current_chat, &client.get_messages(false).await.unwrap())
        .await;

    let mut error_count: u8 = 0;

    info!("Startup complete");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Decline calls
        if let Err(e) = client.decline_call().await {
            error!("Unable to decline call: {:?}", e);
        }

        // See if the current chat has different messages than before
        let current_message = match client.get_messages(false).await {
            Ok(c) => c,
            Err(e) => {
                error!("Unable to get messages: {:?}", e);
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
                error!("Unable to get current chat: {:?}", e);
                error_count += 1;
                if error_count > 10 {
                    return Err(e);
                }
                continue;
            }
        };

        if let Some(unread_messages) = cache.check(&current_chat, &current_message).await {
            for message in unread_messages {
                info!(
                    "{} in {}: {}",
                    message.sender, current_chat, message.content
                );
                let blocking_senders = senders.clone();
                tokio::task::spawn_blocking(move || {
                    blocking_senders
                        .blocking_lock()
                        .retain(|sender| sender.blocking_send(message.clone()).is_ok());
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
                "<file>" => {
                    info!("Sending file!");
                    if let Err(e) = client.go_to_chat(&msg.chat_id).await {
                        error!("Unable to go to chat for file send: {:?}", e);
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                        continue;
                    }
                    if let Err(e) = client.send_file(&msg.content).await {
                        error!("Unable to send file: {:?}", e);
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                        continue;
                    }
                    continue;
                }
                _ => {
                    info!("Sending message: {:?}", msg);
                    if let Err(e) = client.go_to_chat(&msg.chat_id).await {
                        error!("Unable to go to chat for send: {:?}", e);
                        error_count += 1;
                        if error_count > 10 {
                            return Err(e);
                        }
                        continue;
                    }
                    if let Err(e) = client.send_message(&msg.content).await {
                        error!("Unable to send message: {:?}", e);
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
                error!("Unable to get chats: {:?}", e);
                error_count += 1;
                if error_count > 10 {
                    return Err(e);
                }
                continue;
            }
        };
        debug!("Unread chats: {chats:?}");
        chats.retain(|chat| chat.unread || !cache.check_key(&chat.id));
        if !chats.is_empty() {
            if chats[0].click().await.is_err() {
                if let Err(e) = client.refresh().await {
                    error!("Unable to refresh, aborting Holly!");
                    error_count += 1;
                    if error_count > 10 {
                        return Err(e);
                    }
                    return Err(e);
                }
                continue;
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            continue;
        }

        // Until next time *rides motorcycle away*
    }
}

#[tokio::main]
async fn main() {
    println!("Starting Holly core...");

    if std::env::var("RUST_LOG").is_err() {
        println!("Don't forget to initialize the logger with the RUST_LOG env var!!");
    }

    env_logger::init();
    info!("Logger initialized");

    let mut last_error = std::time::Instant::now();
    let mut clear_cookies = false;

    loop {
        if let Err(e) = entry(clear_cookies).await {
            error!("Holly crashed with {:?}", e);
            if last_error.elapsed().as_secs() > 60 {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                info!("Restarting Holly...");
                last_error = std::time::Instant::now();
                clear_cookies = false;
            } else if clear_cookies {
                panic!("Holly has run into an unrecoverable state!")
            } else {
                clear_cookies = true;
            }
        } else {
            clear_cookies = false;
            info!("Holly is restarting...");
        }
    }
}
