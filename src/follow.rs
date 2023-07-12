use std::{sync::Arc, time::Duration};

use crate::misskey::Misskey;
use futures::{StreamExt, TryStreamExt};
use misskey::{streaming::channel::main::MainStreamEvent, ClientExt, StreamingClientExt};
use tokio::time::sleep;

pub async fn monitor_follower(misskey: Arc<Misskey>) -> anyhow::Result<()> {
    'retry: loop {
        let stream_client = misskey.stream().await?;
        let mut stream = stream_client.main_stream().await?;

        while let Some(next) = stream.next().await {
            match next {
                Ok(MainStreamEvent::Followed(user)) => {
                    if let Err(err) = misskey.follow(&user).await {
                        warn!("failed to follow user: {}", err);
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("error while streaming notes: {}. retrying...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue 'retry;
                }
            }
        }
    }
}

pub async fn follow_followers(misskey: Arc<Misskey>) -> anyhow::Result<()> {
    info!("Start following followers");

    let mut followers = misskey.followers(&misskey.me().await?);

    while let Some(follower) = followers.try_next().await? {
        if misskey.is_following(&follower).await?
            || misskey
                .has_pending_follow_request_from_me(&follower)
                .await?
        {
            info!(
                "already following or requested to follow: {}",
                follower.username
            );
            continue;
        }
        info!("following: {}", follower.username);
        if let Err(err) = misskey.follow(&follower).await {
            warn!("failed to follow user: {}", err);
        }
    }

    Ok(())
}
