use std::sync::Arc;

use crate::scheduler::start_scheduler;

#[macro_use]
extern crate log;

mod database;
mod entity;
mod monitor;
mod scheduler;
mod twitter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let twitter = match twitter::Twitter::new().await {
        Ok(twitter) => twitter,
        Err(e) => {
            error!("failed to initialize twitter client: {:#}", e);
            std::process::exit(1);
        }
    };

    info!("screen_name: {}", twitter.screen_name());

    let twitter = Arc::new(twitter);

    let twitter_clone = twitter.clone();
    if let Err(err) = start_scheduler(twitter_clone).await {
        error!("failed to start scheduler: {:#}", err);
    }

    if let Err(err) = monitor::monitor_tweets(twitter).await {
        error!("failed to monitor tweets: {:#}", err);
        std::process::exit(1);
    }
}
