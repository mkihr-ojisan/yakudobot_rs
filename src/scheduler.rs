use anyhow::Context;
use egg_mode::tweet::DraftTweet;
use sea_orm::{prelude::*, QueryOrder};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{database::get_db, entity::yakudo_score, twitter::Twitter};

pub async fn start_scheduler(twitter: Arc<Twitter>) -> anyhow::Result<()> {
    let sched = JobScheduler::new()
        .await
        .context("failed to create scheduler")?;

    let twitter_clone = twitter.clone();
    sched
        .add(
            Job::new_async("0 0 * * * * *", move |_, _| {
                let twitter = twitter_clone.clone();
                Box::pin(async move {
                    log_error(hourly_report(twitter)).await;
                })
            })
            .context("failed to register scheduler job")?,
        )
        .await
        .context("failed to register scheduler job")?;

    let twitter_clone = twitter.clone();
    sched
        .add(
            Job::new_async("0 59 23 * * * *", move |_, _| {
                let twitter = twitter_clone.clone();
                Box::pin(async move {
                    log_error(daily_report(twitter)).await;
                })
            })
            .context("failed to register scheduler job")?,
        )
        .await
        .context("failed to register scheduler job")?;

    let twitter_clone = twitter.clone();
    sched
        .add(
            Job::new_async("0 50 * * * * *", move |_, _| {
                let twitter = twitter_clone.clone();
                Box::pin(async move {
                    log_error(destroy_deleted_tweets(twitter)).await;
                })
            })
            .context("failed to register scheduler job")?,
        )
        .await
        .context("failed to register scheduler job")?;

    sched.start().await.context("failed to start scheduler")?;
    Ok(())
}

async fn log_error(future: impl std::future::Future<Output = anyhow::Result<()>>) {
    if let Err(err) = future.await {
        error!("{}", err);
    }
}

async fn hourly_report(twitter: Arc<Twitter>) -> anyhow::Result<()> {
    trace!("hourly report started");

    let count = yakudo_score::Entity::find()
        .filter(yakudo_score::Column::Date.gt(chrono::Local::now().date().and_hms(0, 0, 0)))
        .count(get_db().await?)
        .await
        .context("failed to count yakudos")?;

    trace!("count: {}", count);

    let message = if count == 0 {
        format!(
            "おいお前ら!早くyakudoしろ!({})",
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        )
    } else {
        format!(
            "本日のyakudo:{}件({})",
            count,
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        )
    };

    trace!("message: {}", message);

    DraftTweet::new(message)
        .send(twitter.token())
        .await
        .context("failed to send hourly report tweet")?;

    Ok(())
}

async fn daily_report(twitter: Arc<Twitter>) -> anyhow::Result<()> {
    trace!("daily report started");

    let yakudos = yakudo_score::Entity::find()
        .filter(yakudo_score::Column::Date.gt(chrono::Local::now().date().and_hms(0, 0, 0)))
        .order_by_desc(yakudo_score::Column::Score)
        .all(get_db().await?)
        .await
        .context("failed to get yakudos")?;

    trace!("yakudos: {:?}", yakudos);

    let message = if let Some(best_yakudo) = yakudos.first() {
        if best_yakudo.score > 0.0 {
            format!(
                "Highest Score:{:.3}\n優勝おめでとう!\nhttps://twitter.com/{}/status/{}",
                best_yakudo.score, best_yakudo.username, best_yakudo.tweet_id
            )
        } else {
            "おい待てや...今日のyakudo...-inf点しか無いやん...".to_string()
        }
    } else {
        "本日のyakudoは...何一つ...出ませんでした...".to_string()
    };

    trace!("message: {}", message);

    DraftTweet::new(message)
        .send(twitter.token())
        .await
        .context("failed to send daily report tweet")?;

    Ok(())
}

async fn destroy_deleted_tweets(twitter: Arc<Twitter>) -> anyhow::Result<()> {
    trace!("destroy deleted tweets started");

    let yakudos = yakudo_score::Entity::find()
        .filter(yakudo_score::Column::Date.gt(chrono::Local::now().date().and_hms(0, 0, 0)))
        .all(get_db().await?)
        .await
        .context("failed to get yakudos")?;

    trace!("yakudos: {:?}", yakudos);

    for yakudo in yakudos {
        trace!("checking tweet: {}", yakudo.tweet_id);

        if egg_mode::tweet::show(yakudo.tweet_id, twitter.token())
            .await
            .is_err()
        {
            trace!(
                "failed to get tweet {}. deleting retweet and database record...",
                yakudo.tweet_id
            );

            egg_mode::tweet::delete(yakudo.retweet_id, twitter.token())
                .await
                .context("failed to delete tweet")?;
            yakudo_score::Entity::delete_by_id(yakudo.id)
                .exec(get_db().await?)
                .await
                .context("failed to delete entity")?;

            trace!("deleted");
        }
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
