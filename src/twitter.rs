use anyhow::Context;
use egg_mode::{KeyPair, Token};

pub struct Twitter {
    token: Token,
    screen_name: String,
}
impl Twitter {
    pub async fn new() -> anyhow::Result<Twitter> {
        let consumer_key = std::env::var("CONSUMER_KEY").context("CONSUMER_KEY is not set")?;
        let consumer_secret =
            std::env::var("CONSUMER_SECRET").context("CONSUMER_SECRET is not set")?;
        let access_token_key =
            std::env::var("ACCESS_TOKEN_KEY").context("ACCESS_TOKEN_KEY is not set")?;
        let access_token_secret =
            std::env::var("ACCESS_TOKEN_SECRET").context("ACCESS_TOKEN_SECRET is not set")?;
        let token = Token::Access {
            consumer: KeyPair::new(consumer_key, consumer_secret),
            access: KeyPair::new(access_token_key, access_token_secret),
        };

        let user = egg_mode::auth::verify_tokens(&token).await?;

        Ok(Twitter {
            token,
            screen_name: user.response.screen_name,
        })
    }

    pub fn screen_name(&self) -> &str {
        &self.screen_name
    }

    pub fn token(&self) -> &Token {
        &self.token
    }
}
