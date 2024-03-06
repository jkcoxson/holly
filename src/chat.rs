// Jackson Coxson

use std::{fmt::{Debug, Formatter}, time::Duration};

use serde::{Deserialize, Serialize};
use thirtyfour::prelude::*;

#[derive(Debug)]
pub struct ChatOption {
    pub id: String,
    pub element: WebElement,
    pub unread: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
    pub chat_id: String,
}

impl ChatOption {
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
                .replace('/', "")
                .replace('t', "");

            // Determine if the unread marker is there
            let unread_marker = chat
                .find(By::XPath(".//div[@aria-label=\"Mark as read\"]"))
                .await;
            let unread = match unread_marker {
                Ok(_) => true,
                Err(_) => false,
            };

            // Add the chat option to the vector
            chat_options_vec.push(ChatOption {
                id,
                element: chat,
                unread,
            });
        }
        Ok(chat_options_vec)
    }

    pub async fn click(&self) -> WebDriverResult<()> {
        self.element.scroll_into_view().await?;
        self.element.click().await?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(())
    }
}

impl ChatMessage {
    pub async fn get(driver: &WebDriver, chat_id: String) -> WebDriverResult<Vec<Self>> {
        // Get the chat container
        let chat_container = driver
            .query(By::XPath(
                "//div[contains(@aria-label, 'conversation') and @role='grid']",
            ))
            .wait(Duration::from_secs(2), Duration::from_millis(100))
            .first()
            .await?;

        // Get all the messages in the chat container
        let messages = chat_container
            .find_all(By::XPath(".//div[@class='x78zum5 xdt5ytf']"))
            .await?;

        let mut res = Vec::new();
        for message in messages {
            match message
                .query(By::XPath(
                    ".//img[@class='x1rg5ohu x5yr21d xl1xv1r xh8yej3']",
                ))
                .wait(Duration::from_millis(15), Duration::from_millis(5))
                .first()
                .await
            {
                Ok(c) => {
                    let sender = c.attr("alt").await?.unwrap();
                    let content = match message
                    .find(By::XPath(".//div[@class='x1gslohp x11i5rnm x12nagc x1mh8g0r x1yc453h x126k92a x18lvrbx']"))
                    .await {
                        Ok(c) => c.text().await?,
                        Err(_) => continue,
                    };

                    res.push(Self {
                        sender,
                        content,
                        chat_id: chat_id.clone(),
                    });
                }
                Err(_) => continue,
            };
        }

        Ok(res)
    }
}

impl Debug for ChatMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Msg")
            .field("sdr", &self.sender)
            .field("msg", &&self.content[0..50])
            .field("id", &self.chat_id)
            .finish()
    }
}
