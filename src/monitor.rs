use std::{sync::Arc, time::Duration};

use crate::{database::get_db, entity, twitter::Twitter};
use anyhow::Context;
use egg_mode::{
    entities::MediaType,
    stream::{FilterLevel, StreamMessage},
    tweet::{DraftTweet, Tweet},
};
use migration::sea_orm::{ActiveModelTrait, ActiveValue};
use opencv::prelude::*;
use tokio::time::sleep;
use tokio_stream::StreamExt;

#[cfg(debug_assertions)]
const SEARCH_KEYWORDS: [&str; 1] = ["#mis1yakudotest"];
#[cfg(not(debug_assertions))]
const SEARCH_KEYWORDS: [&str; 1] = ["#mis1yakudo"];

pub async fn monitor_tweets(twitter: Arc<Twitter>) -> anyhow::Result<()> {
    'retry: loop {
        let mut stream = egg_mode::stream::filter()
            .filter_level(FilterLevel::None)
            .track(SEARCH_KEYWORDS)
            .start(twitter.token());
        info!(
            "Start monitoring tweets with keywords: {:?}",
            SEARCH_KEYWORDS
        );

        while let Some(next) = stream.next().await {
            match next {
                Ok(StreamMessage::Tweet(tweet)) => {
                    let twitter = twitter.clone();
                    tokio::spawn(process_tweet(twitter, tweet));
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("error while streaming tweets: {}. retrying...", e);
                    sleep(Duration::from_secs(5)).await;
                    continue 'retry;
                }
            }
        }
    }
}

async fn process_tweet(twitter: Arc<Twitter>, tweet: Tweet) -> anyhow::Result<()> {
    let tweet_user = tweet.user.as_ref().unwrap();
    let tweet_url = format!(
        "https://twitter.com/{}/status/{}",
        tweet_user.screen_name, tweet.id
    );
    trace!("tweet: {}", tweet_url);

    if tweet_user.screen_name == twitter.screen_name()
        || tweet.retweeted_status.is_some()
        || SEARCH_KEYWORDS
            .iter()
            .any(|keyword| !tweet.text.contains(keyword))
    {
        trace!("tweet does not match the conditions. skipping...");
        return Ok(());
    }

    trace!("tweet: {:?}", tweet);

    let mut message = String::new();
    message.push_str(&chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    message.push_str("\nUser:@");
    message.push_str(&tweet_user.screen_name);
    message.push('\n');

    let mut yakudo_score: f64 = 0.0;

    if let Some(extended_entities) = &tweet.extended_entities {
        let mut final_score = 0.0;
        let mut count = 0;
        let mut is_photo = true;
        for image in &extended_entities.media {
            if image.media_type == MediaType::Video {
                message.push_str("やめろ！クソ動画を投稿するんじゃない!\nScore:-inf\n");
                yakudo_score = 0.0;
                is_photo = false;
                trace!("video found in tweet. aborting...");
                break;
            }

            trace!(
                "calculating yakudo score for image: {}",
                image.media_url_https
            );

            let score = calc_yakudo_score(&image.media_url_https).await?;
            final_score += score;
            count += 1;
            message.push_str(&format!("{}枚目:{:.3}\n", count, score));
            yakudo_score = score;

            trace!("calculated yakudo score for photo {}: {}", count, score);
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
    } else {
        message.push_str("画像が入ってないやん!\nScore:-inf\n");
        trace!("no photo found in tweet. aborting...");
    }

    info!("score: {}", yakudo_score);

    message.push_str(&tweet_url);

    trace!("tweeting: {}", message);

    let response = DraftTweet::new(message).send(twitter.token()).await?;

    let yakudo_score_entity = entity::yakudo_score::ActiveModel {
        username: ActiveValue::Set(tweet_user.screen_name.clone()),
        tweet_id: ActiveValue::Set(tweet.id),
        retweet_id: ActiveValue::Set(response.id),
        score: ActiveValue::Set(yakudo_score),
        date: ActiveValue::Set(chrono::Local::now()),
        ..Default::default()
    };
    trace!("yakudo_score entity: {:#?}", yakudo_score_entity);

    yakudo_score_entity.insert(get_db().await?).await?;

    trace!("finished processing tweet {}", tweet_url);
    Ok(())
}

async fn calc_yakudo_score(url: &str) -> anyhow::Result<f64> {
    let image_bytes = reqwest::get(url).await?.bytes().await?.to_vec();
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
