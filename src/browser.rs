// Jackson Coxson

use std::process::Stdio;

use log::{error, info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use thirtyfour::prelude::*;
use tokio::process::{Child, Command};

use crate::config::Config;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/87.0.4280.88 Safari/537.36";

pub struct Browser {
    driver: WebDriver,
    _gecko: Child,
}

#[derive(Serialize, Deserialize)]
struct JsonCookie {
    name: String,
    value: String,
}

impl Browser {
    pub async fn new(config: &Config) -> Result<Self, WebDriverResult<()>> {
        let _gecko = launch_driver(&config.gecko.path, config.gecko.port);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut caps = DesiredCapabilities::firefox();
        caps.add_firefox_arg("--disable-infobars").unwrap();
        caps.add_firefox_arg("--no-sandbox").unwrap();
        caps.add_firefox_arg("--disable-application-cache").unwrap();
        caps.add_firefox_arg("--disable-gpu").unwrap();
        caps.add_firefox_arg("--disable-dev-shm-usage").unwrap();
        caps.add_firefox_arg("--start-maximized").unwrap();
        caps.add_firefox_arg("--disable-extensions").unwrap();
        caps.add_firefox_arg("--window-size=1920,1080").unwrap();
        caps.add_firefox_arg(&format!("user-agent={USER_AGENT}"))
            .unwrap();

        if config.gecko.headless {
            caps.add_firefox_arg("--headless").unwrap();
            caps.add_firefox_arg("--disable-gpu").unwrap();
        }

        let driver = WebDriver::new("http://localhost:4444", caps).await.unwrap();

        driver.goto("https://messenger.com").await.unwrap();

        Ok(Self { driver, _gecko })
    }

    /// Logs into Messenger. This will only work if we're not already logged in
    pub async fn login(&self, username: &str, password: &str) -> WebDriverResult<()> {
        self.driver.goto("https://messenger.com").await?;
        let email_input = self
            .driver
            .query(By::Id("email"))
            .wait(
                std::time::Duration::from_secs(10),
                std::time::Duration::from_millis(100),
            )
            .and_clickable()
            .first()
            .await?;

        let password_input = self
            .driver
            .query(By::Id("pass"))
            .wait(
                std::time::Duration::from_secs(10),
                std::time::Duration::from_millis(100),
            )
            .and_clickable()
            .first()
            .await?;

        let login_button = self
            .driver
            .query(By::Id("loginbutton"))
            .wait(
                std::time::Duration::from_secs(10),
                std::time::Duration::from_millis(100),
            )
            .first()
            .await?;

        email_input.send_keys(username).await?;
        password_input.send_keys(password).await?;
        login_button.click().await?;

        Ok(())
    }

    /// Checks for the xs cookie (token) and the presence of the 'Chats' h1 element. It loads before XHR requests are made
    pub async fn is_logged_in(&self) -> bool {
        // Does the xs cookie exist?
        self.driver.get_named_cookie("xs").await.is_ok()
            && self
                .driver
                .find(By::XPath("//a[@aria-label=\"New message\"]"))
                .await
                .is_ok()
    }

    /// Enters the dumb e2ee pin Facebook is shoving down everyones' throats
    pub async fn enter_e2ee_pin(&self, pin: String) {
        if let Ok(pin_input) = self
            .driver
            .query(By::XPath(
                "//input[@id='mw-numeric-code-input-prevent-composer-focus-steal']",
            ))
            .wait(
                std::time::Duration::from_secs(5),
                std::time::Duration::from_millis(500),
            )
            .first()
            .await
        {
            // enter that pin
            info!("Entering e2ee pin");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if let Err(e) = pin_input.send_keys(pin).await {
                warn!("Failed to send keys to pin input: {e:?}");
            }
        } else {
            warn!("Failed to get the pin box, but a e2ee pin was supplied via config!");
        }
    }

    /// Gets all the chats on the side bar. Includes whether or not they are unread.
    pub async fn get_chats(&self) -> WebDriverResult<Vec<crate::chat::ChatOption>> {
        crate::chat::ChatOption::get_all(&self.driver).await
    }

    /// Navigates the browser to the chat with the given id.
    /// Attempts to find it on the side bar to click that object.
    /// If it's not found, it will just navigate via URL.
    pub async fn go_to_chat(&self, id: &str) -> WebDriverResult<()> {
        self.decline_call().await.unwrap();
        let chats = self.get_chats().await?;
        match chats.iter().find(|c| c.id == id) {
            Some(chat) => {
                chat.click().await?;
            }
            None => {
                // Manually go
                self.driver
                    .goto(format!("https://www.messenger.com/t/{}", id))
                    .await?;
            }
        }
        Ok(())
    }

    /// Declines a Messenger call on the browser
    pub async fn decline_call(&self) -> WebDriverResult<()> {
        // Get the decline object if it exists
        // aria-lable = "Decline"
        let decline = self
            .driver
            .find(By::XPath("//div[@aria-label=\"Decline\"]"))
            .await;

        if let Ok(d) = decline {
            info!("Declining call");
            d.click().await?;
        }
        Ok(())
    }

    /// Refreshes the tab
    pub async fn refresh(&self) -> WebDriverResult<()> {
        self.driver.refresh().await?;
        Ok(())
    }

    /// Takes a screenshot and saves it to logs/timestamp.png
    pub async fn screenshot_log(&self) -> WebDriverResult<()> {
        let b = self.driver.screenshot_as_png().await?;
        let timestamp = chrono::offset::Local::now().to_string();

        // Create the log folder if not created
        if let Err(e) = tokio::fs::create_dir_all("logs").await {
            error!("Could not create logs folder: {:?}", e);
            return Err(WebDriverError::CustomError(
                "Could not create logs folder".to_string(),
            ));
        }

        match tokio::fs::File::create(format!("logs/{timestamp}-log.png")).await {
            Ok(mut file) => {
                if tokio::io::AsyncWriteExt::write_all(&mut file, &b)
                    .await
                    .is_err()
                {
                    error!("Could not write screenshot data to file");
                    return Err(WebDriverError::CustomError(
                        "Could not write screenshot data to file".to_string(),
                    ));
                }
                Ok(())
            }
            Err(e) => {
                error!("Could not create file to save screenshot: {:?}", e);
                Err(WebDriverError::CustomError(
                    "Could not create file to save screenshot".to_string(),
                ))
            }
        }
    }

    /// Takes a snapshot of the page HTML and saves to to logs/timestamp.html
    pub async fn html_log(&self) -> WebDriverResult<()> {
        let html = self.driver.source().await?;
        let timestamp = chrono::offset::Local::now().to_string();

        // Create the log folder if not created
        if let Err(e) = tokio::fs::create_dir_all("logs").await {
            error!("Could not create logs folder: {:?}", e);
            return Err(WebDriverError::CustomError(
                "Could not create logs folder".to_string(),
            ));
        }

        if let Ok(mut file) = tokio::fs::File::create(format!("logs/{timestamp}-log.html")).await {
            if tokio::io::AsyncWriteExt::write_all(&mut file, html.as_bytes())
                .await
                .is_err()
            {
                error!("Could not write html data to file");
                return Err(WebDriverError::CustomError(
                    "Could not write html data to file".to_string(),
                ));
            }
            Ok(())
        } else {
            error!("Could not create file to save html");
            Err(WebDriverError::CustomError(
                "Could not create file to save html".to_string(),
            ))
        }
    }

    /// Gets the ID of the current chat
    pub async fn get_current_chat(&self) -> WebDriverResult<String> {
        let current_url = self.driver.current_url().await?;
        let id = current_url
            .path()
            .split('/')
            .filter(|x| !x.is_empty())
            .last()
            .unwrap();
        Ok(id.to_string())
    }

    /// Gets the list of all the messages in the current chat
    pub async fn get_messages(&self, last: bool) -> WebDriverResult<Vec<crate::chat::ChatMessage>> {
        crate::chat::ChatMessage::get(&self.driver, self.get_current_chat().await?, last).await
    }

    /// Sends a message to the current chat
    pub async fn send_message(&self, message: &str) -> WebDriverResult<()> {
        self.decline_call().await.unwrap();

        let chat_bar = match self
            .driver
            .query(By::XPath("//div[@role='textbox']"))
            .wait(
                std::time::Duration::from_secs(5),
                std::time::Duration::from_millis(100),
            )
            .first()
            .await
        {
            Ok(c) => c,
            Err(_) => {
                warn!("Unable to get sender box by textbox role");
                self.driver
                    .find(By::XPath("//div[@aria-label='Message']"))
                    .await?
            }
        };
        chat_bar.click().await?;

        let mut rand_gen = rand::thread_rng();
        for c in message.chars() {
            self.decline_call().await.unwrap();
            let x = rand_gen.gen_range(1..=30);
            if x == 7 {
                for asdf in "asdf".chars() {
                    chat_bar.send_keys(String::from(asdf)).await?;
                    tokio::time::sleep(std::time::Duration::from_millis(
                        rand_gen.gen_range(10..=20),
                    ))
                    .await;
                }
                for _ in 0..4 {
                    chat_bar.send_keys(Key::Backspace + "").await?;
                    tokio::time::sleep(std::time::Duration::from_millis(
                        rand_gen.gen_range(10..=20),
                    ))
                    .await;
                }
            }
            chat_bar.send_keys(String::from(c)).await?;
            tokio::time::sleep(std::time::Duration::from_millis(
                rand_gen.gen_range(10..=20),
            ))
            .await;
        }
        chat_bar.send_keys(Key::Enter + "").await?;

        if let Ok(send_button) = self
            .driver
            .find(By::XPath("//div[@aria-label='Press enter to send']"))
            .await
        {
            let _ = send_button.click().await;
        }

        Ok(())
    }

    pub async fn send_file(&self, path: &str) -> WebDriverResult<()> {
        self.decline_call().await.unwrap();

        let chat_bar = match self
            .driver
            .query(By::XPath("//div[@role='textbox']"))
            .wait(
                std::time::Duration::from_secs(5),
                std::time::Duration::from_millis(100),
            )
            .first()
            .await
        {
            Ok(c) => c,
            Err(_) => {
                warn!("Unable to get sender box by textbox role");
                self.driver
                    .find(By::XPath("//div[@aria-label='Message']"))
                    .await?
            }
        };
        chat_bar.click().await?;

        let ret = self
            .driver
            .execute(
                include_str!("drop.js"),
                vec![
                    chat_bar.to_json()?,
                    Value::Number(Number::from(0)),
                    Value::Number(Number::from(0)),
                ],
            )
            .await?
            .element()?;

        ret.send_keys(path).await?;

        // Detect an invalid file format
        if let Ok(dialogue) = self
            .driver
            .find(By::XPath("//div[@aria-label='Invalid file format']"))
            .await
        {
            warn!("File upload failed: invalid file format!");
            // Close the box
            dialogue
                .find(By::XPath("//div[@aria-label='Close']"))
                .await?
                .click()
                .await?;
        }

        // Detect a file upload
        if let Ok(dialogue) = self
            .driver
            .find(By::XPath("//div[@aria-label='Failed to upload files']"))
            .await
        {
            warn!("File upload failed! (Is the file below 25 MB?)");
            // Close the box
            dialogue
                .find(By::XPath("//div[@aria-label='Close']"))
                .await?
                .click()
                .await?;
        }

        chat_bar.click().await?;
        chat_bar.send_keys(Key::Enter + "").await?;

        if let Ok(send_button) = self
            .driver
            .find(By::XPath("//div[@aria-label='Press enter to send']"))
            .await
        {
            let _ = send_button.click().await;
        }

        Ok(())
    }

    /// Dumps the cookies to cookies.json so we don't have to login every time
    pub async fn dump_cookies(&self) -> WebDriverResult<()> {
        let cookies = self.driver.get_all_cookies().await?;
        let mut file = match std::fs::File::create("cookies.json") {
            Ok(file) => file,
            Err(e) => {
                error!("Failed to create cookies.json: {}", e);
                return Err(WebDriverError::CustomError(format!(
                    "Failed to create cookies.json: {}",
                    e
                )));
            }
        };
        let json_cookies: Vec<JsonCookie> = cookies
            .iter()
            .map(|c| JsonCookie {
                name: c.name().to_owned(),
                value: c.value().to_owned(),
            })
            .collect();
        serde_json::to_writer_pretty(&mut file, &json_cookies).unwrap();
        Ok(())
    }

    /// Loads the cookies from cookies.json so we don't have to login every time
    pub async fn load_cookies(&self) -> WebDriverResult<()> {
        let mut file = match std::fs::File::open("cookies.json") {
            Ok(file) => file,
            Err(_) => {
                warn!("No cookies.json file found");
                return Ok(());
            }
        };
        let mut json_cookies: Vec<JsonCookie> = serde_json::from_reader(&mut file).unwrap();
        for cookie in json_cookies.drain(..) {
            self.driver
                .add_cookie(
                    Cookie::build(cookie.name, cookie.value)
                        .path("/")
                        .domain("messenger.com")
                        .finish(),
                )
                .await?;
        }
        self.driver.refresh().await?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        Ok(())
    }

    /// Wipes the cookies
    pub async fn delete_cookies(&self) -> WebDriverResult<()> {
        self.driver.delete_all_cookies().await?;
        Ok(())
    }
}

fn launch_driver(path: &str, port: u16) -> Child {
    Command::new(path)
        .arg("-p")
        .arg(port.to_string())
        .kill_on_drop(true)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .unwrap()
}
