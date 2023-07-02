use std::ops::Deref;

use anyhow::Context;
use misskey::{
    model::{id::Id, note::Note, user::User},
    ClientExt, HttpClient, WebSocketClient,
};

pub struct Misskey {
    instance: String,
    token: String,
    client: HttpClient,
    user_id: Id<User>,
    secure: bool,
}
impl Misskey {
    pub async fn new() -> anyhow::Result<Misskey> {
        info!("initializing misskey client...");

        let instance = std::env::var("INSTANCE").context("INSTANCE is not set")?;
        let token = std::env::var("TOKEN").context("TOKEN is not set")?;
        let secure = std::env::var("SECURE")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()?;

        let api_endpoint = if secure {
            format!("https://{}/api/", instance)
        } else {
            format!("http://{}/api/", instance)
        };

        let client = HttpClient::builder(&*api_endpoint).token(&token).build()?;

        let me = client.me().await?;
        info!("username: {}, id: {}", me.username, me.id);

        Ok(Misskey {
            instance,
            token,
            client,
            user_id: me.id,
            secure,
        })
    }

    pub async fn stream(&self) -> anyhow::Result<WebSocketClient> {
        let websocket_endpoint = if self.secure {
            format!("wss://{}/streaming", self.instance)
        } else {
            format!("ws://{}/streaming", self.instance)
        };

        Ok(WebSocketClient::builder(&*websocket_endpoint)
            .token(&self.token)
            .connect()
            .await?)
    }

    pub fn get_note_url(&self, note: &Note) -> String {
        if self.secure {
            format!("https://{}/notes/{}", self.instance, note.id)
        } else {
            format!("http://{}/notes/{}", self.instance, note.id)
        }
    }

    pub fn user_id(&self) -> Id<User> {
        self.user_id
    }
}

impl Deref for Misskey {
    type Target = HttpClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
