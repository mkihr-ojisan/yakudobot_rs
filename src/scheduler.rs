use anyhow::Context;
use chrono::Timelike;
use egg_mode::tweet::DraftTweet;
use sea_orm::{prelude::*, QueryOrder};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::time::sleep;

use crate::{database::get_db, entity::yakudo_score, twitter::Twitter};

pub struct Job {
    hour: Option<u32>,
    minute: Option<u32>,
    job: Box<dyn FnMut() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>,
}
impl Job {
    pub fn new(
        hour: impl Into<Option<u32>>,
        minute: impl Into<Option<u32>>,
        job: impl FnMut() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            hour: hour.into(),
            minute: minute.into(),
            job: Box::new(job),
        }
    }
}
pub struct Scheduler {
    jobs: Vec<Job>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self { jobs: vec![] }
    }
    pub fn add(&mut self, job: Job) {
        self.jobs.push(job);
    }
    pub fn start(self) {
        tokio::spawn(async move {
            let mut jobs = self.jobs;

            loop {
                let now = chrono::Local::now();
                sleep(Duration::from_millis(
                    (now.with_minute(now.minute() + 1)
                        .unwrap()
                        .with_second(0)
                        .unwrap()
                        - now)
                        .num_milliseconds() as u64,
                ))
                .await;

                let now = chrono::Local::now();
                for job in &mut jobs {
                    if job.hour.map(|hour| hour == now.hour()).unwrap_or(true)
                        && job
                            .minute
                            .map(|minute| minute == now.minute())
                            .unwrap_or(true)
                    {
                        tokio::spawn((job.job)());
                    }
                }
            }
        });
    }
}

pub async fn start_scheduler(twitter: Arc<Twitter>) -> anyhow::Result<()> {
    let mut sched = Scheduler::new();

    let twitter_clone = twitter.clone();
    sched.add(Job::new(None, 0, move || {
        let twitter = twitter_clone.clone();
        Box::pin(async move {
            log_error(hourly_report(twitter)).await;
        })
    }));

    let twitter_clone = twitter.clone();
    sched.add(Job::new(23, 59, move || {
        let twitter = twitter_clone.clone();
        Box::pin(async move {
            log_error(daily_report(twitter)).await;
        })
    }));

    let twitter_clone = twitter;
    sched.add(Job::new(None, 50, move || {
        let twitter = twitter_clone.clone();
        Box::pin(async move {
            log_error(destroy_deleted_tweets(twitter)).await;
        })
    }));

    sched.start();

    Ok(())
}

async fn log_error(future: impl std::future::Future<Output = anyhow::Result<()>>) {
    if let Err(err) = future.await {
        error!("{:#}", err);
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
