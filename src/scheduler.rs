use anyhow::Context;
use chrono::Timelike;
use misskey::{
    model::{id::Id, note::Note},
    ClientExt,
};
use sea_orm::{prelude::*, QueryOrder};
use std::{future::Future, ops::Add, pin::Pin, sync::Arc, time::Duration};
use tokio::time::sleep;

use crate::{database::get_db, entity::yakudo_score, misskey::Misskey};

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
                    (now.add(chrono::Duration::minutes(1))
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

pub async fn start_scheduler(misskey: Arc<Misskey>) -> anyhow::Result<()> {
    let mut sched = Scheduler::new();

    let misskey_clone = misskey.clone();
    sched.add(Job::new(23, 59, move || {
        let misskey = misskey_clone.clone();
        Box::pin(async move {
            log_error(daily_report(misskey)).await;
        })
    }));

    let misskey_clone = misskey;
    sched.add(Job::new(None, 50, move || {
        let misskey = misskey_clone.clone();
        Box::pin(async move {
            log_error(destroy_deleted_notes(misskey)).await;
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

async fn daily_report(misskey: Arc<Misskey>) -> anyhow::Result<()> {
    info!("daily report started");

    let yakudos = yakudo_score::Entity::find()
        .filter(
            yakudo_score::Column::Date.gt(chrono::Local::now().date_naive().and_hms_opt(0, 0, 0)),
        )
        .order_by_desc(yakudo_score::Column::Score)
        .all(get_db().await?)
        .await
        .context("failed to get yakudos")?;

    info!("yakudos: {:?}", yakudos);

    if let Some(best_yakudo) = yakudos.first() {
        if best_yakudo.score > 0.0 {
            let message = format!("Highest Score:{:.3}\n優勝おめでとう!", best_yakudo.score);
            misskey
                .quote(best_yakudo.note_id.parse::<Id<Note>>()?, &message)
                .await?;
            info!("message: {}", message);
        } else {
            let message = "おい待てや...今日のyakudo...-inf点しか無いやん...";
            misskey.create_note(message).await?;
            info!("message: {}", message);
        }
    } else {
        let message = "本日のyakudoは...何一つ...出ませんでした...";
        misskey.create_note(message).await?;
        info!("message: {}", message);
    }

    Ok(())
}

async fn destroy_deleted_notes(misskey: Arc<Misskey>) -> anyhow::Result<()> {
    info!("destroy deleted notes started");

    let yakudos = yakudo_score::Entity::find()
        .filter(
            yakudo_score::Column::Date.gt(chrono::Local::now().date_naive().and_hms_opt(0, 0, 0)),
        )
        .all(get_db().await?)
        .await
        .context("failed to get yakudos")?;

    info!("yakudos: {:?}", yakudos);

    for yakudo in yakudos {
        info!("checking note: {}", yakudo.note_id);

        let note_id = yakudo.note_id.parse::<Id<Note>>()?;
        if misskey.get_note(note_id).await.is_err() {
            info!(
                "failed to get note {}. deleting quote and database record...",
                yakudo.note_id
            );

            misskey
                .delete_note(note_id)
                .await
                .context("failed to delete note")?;
            yakudo_score::Entity::delete_by_id(yakudo.id)
                .exec(get_db().await?)
                .await
                .context("failed to delete entity")?;

            info!("deleted");
        }
        sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
