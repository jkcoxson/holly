// Jackson Coxson

use std::{
    fmt::{Debug, Formatter},
    time::Duration,
};

use log::{debug, warn};
use serde::{Deserialize, Serialize};
use thirtyfour::prelude::*;

/// A chat found on the sidebar.
/// Includes whether or not the chat is unread.
pub struct ChatOption {
    pub id: String,
    pub element: WebElement,
    pub unread: bool,
}

/// A message found in a chat.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub chat_id: String,
}

impl ChatOption {
    /// Gets all the chats in the sidebar
    pub async fn get_all(driver: &WebDriver) -> WebDriverResult<Vec<ChatOption>> {
        // Get the chats object
        let chats_object = driver
            .query(By::XPath("//div[@aria-label=\"Chats\" and @role=\"grid\"]"))
            .wait(Duration::from_secs(15), Duration::from_millis(100))
            .first()
            .await?;

        // Get all the chat options
        let chat_options = chats_object
            .find_all(By::XPath(".//div[@class=\"x78zum5 xdt5ytf\"]"))
            .await?;

        // Create a vector to store the chat options
        let mut chat_options_vec: Vec<ChatOption> = Vec::new();

        for chat in chat_options {
            // Get chat ID
            let link_object = chat.find(By::XPath(".//a[@role=\"link\"]")).await?;
            let id = link_object
                .attr("href")
                .await?
                .unwrap()
                .replace(['/', 't'], "").replace("e2ee", "");

            // Determine if the unread marker is there
            let unread_marker = chat
                .find(By::XPath(".//span[@class=\"x6s0dn4 xzolkzo x12go9s9 x1rnf11y xprq8jg x9f619 x3nfvp2 xl56j7k xwnonoy x170jfvy x1fsd2vl\"]"))
                .await;
            let unread = unread_marker.is_ok();

            // Add the chat option to the vector
            chat_options_vec.push(ChatOption {
                id,
                element: chat,
                unread,
            });
        }
        Ok(chat_options_vec)
    }

    /// Clicks on the sidebar, thereby navigating to the chat
    pub async fn click(&self) -> WebDriverResult<()> {
        self.element.scroll_into_view().await?;
        self.element.click().await?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        Ok(())
    }
}

impl ChatMessage {
    /// Gets all the chat messages in the current chat
    pub async fn get(
        driver: &WebDriver,
        chat_id: String,
        last: bool,
    ) -> WebDriverResult<Vec<Self>> {
        // Get the chat container
        let chat_container = driver
            .query(By::XPath(
                "//div[contains(@aria-label, 'conversation') and @role='grid']",
            ))
            .wait(Duration::from_secs(2), Duration::from_millis(100))
            .first()
            .await?;

        // Get all the messages in the chat container
        let mut tries = 0;
        let messages = loop {
            debug!("Getting chat messages from container");
            let messages = chat_container
                .find_all(By::XPath(".//div[@class='x78zum5 xdt5ytf']"))
                .await?;
            if messages.len() > 13 || tries > 5 {
                if last && !messages.is_empty() {
                    break vec![messages.last().unwrap().to_owned()];
                }
                break messages;
            }
            debug!("Failed to get at least 13 chat messages, trying again...");
            tries += 1;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await
        };
        if messages.is_empty() {
            warn!("Collected no messages!");
        }

        let mut res = Vec::new();
        let mut homeless = Vec::new();
        for message in messages {
            match message
                .query(By::XPath(
                    ".//div[@class='html-div xexx8yu x4uap5 x18d9i69 xkhd6sd x1gslohp x11i5rnm x12nagc x1mh8g0r x1yc453h x126k92a x18lvrbx']",
                ))
                .wait(Duration::from_millis(15), Duration::from_millis(5))
                .first()
                .await
            {
                Ok(c) => {
                    let content = c.text().await?;
                    
                    let sender = match message.query(By::XPath(".//img[@class='x1rg5ohu x5yr21d xl1xv1r xh8yej3']"))
                    .wait(Duration::from_millis(15), Duration::from_millis(5))
                    .first().await {
                        Ok(c) => c.attr("alt").await?.unwrap(),
                        Err(e) => {
                            // If the same user sends a message twice in a row,
                            // there will be no sender detected in the HTML.
                            // Store the messages in the homeless camp until we get one.
                            homeless.push(content);
                            debug!("Unable to get sender from the image alt: {e:?}");
                            continue;
                        },
                    };

                    // We have a sender for the homeless messages
                    for h in homeless.drain(..) {
                        res.push(Self {
                            sender: sender.clone(),
                            content: h,
                            chat_id: chat_id.clone()
                        })
                    }

                    res.push(Self {
                        sender,
                        content,
                        chat_id: chat_id.clone(),
                    });
                }
                Err(e) => {
                    // Check if the message is a single emoji
                    debug!("Unable to get message from the element! {e:?}");

                        match message.query(By::XPath(".//img[@class='xz74otr']")).wait(Duration::from_millis(15), Duration::from_millis(5)).first().await {
                            Ok(o) => {
                                if let Ok(Some(attr)) = o.attr("alt").await {
                                    let content = attr.chars().filter(|&c| c != '\u{fe0f}').collect();
                                    let sender = match message.query(By::XPath(".//img[@class='x1rg5ohu x5yr21d xl1xv1r xh8yej3']"))
                                    .wait(Duration::from_millis(15), Duration::from_millis(5))
                                    .first().await {
                                        Ok(c) => c.attr("alt").await?.unwrap(),
                                        Err(e) => {
                                            homeless.push(content);
                                            debug!("Unable to get sender from the image alt: {e:?}");
                                            continue;
                                        },
                                    };
                                    for h in homeless.drain(..) {
                                        res.push(Self {
                                            sender: sender.clone(),
                                            content: h,
                                            chat_id: chat_id.clone()
                                        })
                                    }
                                    res.push(Self {
                                        sender,
                                        content,
                                        chat_id: chat_id.clone(),
                                    });
                                } else {
                                    debug!("Emoji object has no attribute");
                                }
                            }
                            Err(e) => {
                                debug!("No emoji object on message: {e:?}");
                            }
                        }

                    continue;
                }
            };
        }

        Ok(res)
    }

    /// Removes special characters that can't be sent into Messenger
    pub fn clean(&mut self) {
        self.content = unidecode::unidecode(&self.content);
    }
}

impl Debug for ChatOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chat")
            .field("id", &self.id)
            .field("unread", &self.unread)
            .finish()
    }
}

impl Debug for ChatMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let msg_chars = &self.content.chars().collect::<Vec<char>>();
        let msg = if msg_chars.len() > 50 {
            format!("{}...", &msg_chars[..50].iter().collect::<String>())
        } else {
            self.content.to_string()
        };
        f.debug_struct("Msg")
            .field("sdr", &self.sender)
            .field("msg", &msg)
            .field("chat_id", &self.chat_id)
            .finish()
    }
}
