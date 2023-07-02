use std::{sync::Arc, time::Duration};

use crate::{database::get_db, entity, misskey::Misskey};
use anyhow::Context;
use futures::StreamExt;
use migration::sea_orm::{ActiveModelTrait, ActiveValue};
use misskey::{model::note::Note, ClientExt, StreamingClientExt};
use opencv::prelude::*;
use reqwest::Url;
use tokio::time::sleep;

#[cfg(debug_assertions)]
const SEARCH_HASHTAG: &str = "mis1yakudotest";
#[cfg(not(debug_assertions))]
const SEARCH_HASHTAG: &str = "mis1yakudo";

pub async fn monitor_notes(misskey: Arc<Misskey>) -> anyhow::Result<()> {
    'retry: loop {
        let stream_client = misskey.stream().await?;
        let mut stream = stream_client.hashtag_timeline(SEARCH_HASHTAG).await?;
        info!("Start monitoring notes with hashtag: #{:?}", SEARCH_HASHTAG);

        while let Some(next) = stream.next().await {
            match next {
                Ok(note) => {
                    if let Err(err) = process_note(misskey.clone(), note).await {
                        warn!("error while processing note: {}. retrying...", err);
                    }
                }
                Err(e) => {
                    warn!("error while streaming notes: {}. retrying...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue 'retry;
                }
            }
        }
    }
}

#[async_recursion::async_recursion]
async fn process_note(misskey: Arc<Misskey>, note: Note) -> anyhow::Result<()> {
    if let Some(reply_id) = &note.reply_id {
        let note = misskey
            .get_note(*reply_id)
            .await
            .context("failed to get the note that this note is replying to")?;
        return process_note(misskey, note).await;
    }

    let note_url = misskey.get_note_url(&note);
    info!("note: {}", note_url);

    if note.user.id == misskey.user_id() || note.renote_id.is_some() {
        info!("note does not match the conditions. skipping...");
        return Ok(());
    }

    info!("note: {:?}", note);

    let mut message = String::new();
    message.push_str(&chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    message.push_str("\nUser:@");
    message.push_str(&note.user.username);
    message.push('\n');

    let mut yakudo_score: f64 = 0.0;

    if note.files.is_empty() {
        message.push_str("画像が入ってないやん!\nScore:-inf\n");
        info!("no photo found in note. aborting...");
    } else {
        let mut final_score = 0.0;
        let mut count = 0;
        let mut is_photo = true;
        for file in &note.files {
            match file.type_.type_() {
                mime::VIDEO => {
                    message.push_str("やめろ！クソ動画を投稿するんじゃない!\nScore:-inf\n");
                    yakudo_score = 0.0;
                    is_photo = false;
                    info!("video found in note. aborting...");
                    break;
                }
                mime::IMAGE => {
                    let url = if let Some(url) = &file.url {
                        url
                    } else {
                        info!("file url not found. skipping...");
                        continue;
                    };
                    info!("calculating yakudo score for image: {}", url);

                    let score = calc_yakudo_score(url).await?;
                    final_score += score;
                    count += 1;
                    message.push_str(&format!("{}枚目:{:.3}\n", count, score));
                    yakudo_score = score;

                    info!("calculated yakudo score for photo {}: {}", count, score);
                }
                _ => {
                    info!("file type is not image. skipping...");
                    continue;
                }
            }
        }
        if is_photo {
            final_score /= count as f64;
            if final_score >= 150.0 {
                message.push_str("GoodYakudo!\n");
            } else {
                message.push_str("もっとyakudoしろ！\n");
            }
            message.push_str(&format!("Score:{:.3}\n", final_score));
        }
    }

    info!("score: {}", yakudo_score);

    info!("noting: {}", message);

    let response = misskey.quote(&note, message).await?;

    let yakudo_score_entity = entity::yakudo_score::ActiveModel {
        username: ActiveValue::Set(note.user.username),
        note_id: ActiveValue::Set(note.id.to_string()),
        quote_id: ActiveValue::Set(response.id.to_string()),
        score: ActiveValue::Set(yakudo_score),
        date: ActiveValue::Set(chrono::Local::now()),
        ..Default::default()
    };
    info!("yakudo_score entity: {:#?}", yakudo_score_entity);

    yakudo_score_entity.insert(get_db().await?).await?;

    info!("finished processing note {}", note_url);
    Ok(())
}

async fn calc_yakudo_score(url: &Url) -> anyhow::Result<f64> {
    let image_bytes = reqwest::get(url.clone()).await?.bytes().await?.to_vec();
    let image = opencv::imgcodecs::imdecode(
        &opencv::core::Vector::<u8>::from_slice(&image_bytes),
        opencv::imgcodecs::IMREAD_COLOR,
    )?;
    let mut result = opencv::core::Mat::default();
    opencv::imgproc::laplacian(
        &image,
        &mut result,
        opencv::core::CV_64F,
        1,
        1.0,
        0.0,
        opencv::core::BORDER_DEFAULT,
    )
    .context("failed to calculate yakudo score")?;

    let sum = result
        .iter::<opencv::core::Point3_<f64>>()
        .unwrap()
        .map(|(_, p)| p.x + p.y + p.z)
        .sum::<f64>();
    let mean = sum / (result.rows() * result.cols() * 3) as f64;
    let variance = result
        .iter::<opencv::core::Point3_<f64>>()
        .unwrap()
        .map(|(_, p)| (p.x - mean).powi(2) + (p.y - mean).powi(2) + (p.z - mean).powi(2))
        .sum::<f64>()
        / (result.rows() * result.cols() * 3) as f64;

    let score = 1.0 / variance * 10000.0;

    Ok(score)
}
