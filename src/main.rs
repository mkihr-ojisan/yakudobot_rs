use std::sync::Arc;

use crate::scheduler::start_scheduler;

#[macro_use]
extern crate log;

mod database;
mod entity;
mod follow;
mod misskey;
mod monitor;
mod scheduler;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let misskey = match misskey::Misskey::new().await {
        Ok(misskey) => misskey,
        Err(e) => {
            error!("failed to initialize misskey client: {:#}", e);
            std::process::exit(1);
        }
    };

    let misskey = Arc::new(misskey);

    let misskey_clone = misskey.clone();
    if let Err(err) = start_scheduler(misskey_clone).await {
        error!("failed to start scheduler: {:#}", err);
    }

    let misskey_clone = misskey.clone();
    tokio::spawn(async move {
        if let Err(err) = monitor::monitor_notes(misskey_clone).await {
            error!("failed to monitor notes: {:#}", err);
            std::process::exit(1);
        }
    });

    if let Err(err) = follow::monitor_follower(misskey).await {
        error!("failed to monitor follower: {:#}", err);
        std::process::exit(1);
    }
}
