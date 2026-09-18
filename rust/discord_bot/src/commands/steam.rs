use crate::utils::utils::{read_json_file, write_json_file, read_hashmap_json_file};
use poise::serenity_prelude::ChannelId;
use poise::serenity_prelude as serenity;
use reqwest::Client;
use std::collections::HashMap;
use chrono::{NaiveDateTime, Datelike};
const STEAM_DATA_FOLDER: &str = "../../data/steam_data";
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const CHANNEL_ID: u64 = 411692824027725824;  // Replace with your Channel ID
use crate::{Context, Error, Data};
use crate::env;
use rand::seq::IndexedRandom;
use std::sync::Arc;
use chrono::{Local, NaiveTime, Duration as ChronoDuration};
use chrono::TimeZone;


use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameCache {
    game: GameInfo,
    time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameInfo {
    appid: i64,
    last_modified: i64,
    name: String,
    price_change_number: i64
}

#[poise::command(prefix_command, guild_only)]
pub async fn game(
    ctx: Context<'_>
) -> Result<(), Error> {
    let _ = game_of_day(&ctx.serenity_context().http, &ctx.channel_id().to_channel(&ctx.serenity_context()).await.unwrap()).await;
    Ok(())
}


async fn game_of_day(http: &serenity::Http, channel: &serenity::Channel) -> Result<(), Error> {
    async fn fetch_data(url: &str) -> Option<serde_json::Value> {
        match Client::new().get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => Some(json),
                        Err(e) => {
                            println!("Error parsing JSON from Steam API: {}", e);
                            None
                        }
                    }
                } else {
                    println!("Error fetching data from Steam API: {}", response.status());
                    None
                }
            }
            Err(e) => {
                println!("Error fetching data from Steam API: {}", e);
                None
            }
        }
    }

    let guild_id = channel.clone().guild().unwrap().guild_id.get();
    let game_of_the_day_cache = read_hashmap_json_file(&format!("{}/game_of_the_day.json", STEAM_DATA_FOLDER)).await.unwrap_or_default();
    let cached_game: Option<GameCache> = game_of_the_day_cache
        .get(&guild_id.to_string())
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let steamapi_key = env::var("STEAMAPI_KEY").unwrap_or_default();

    if let Some(cached_game) = cached_game {
        let cached_time = {
            NaiveDateTime::parse_from_str(&cached_game.time, DATETIME_FORMAT).unwrap()
        };
        if Local::now().naive_local().date() == cached_time.date() {
           let game = &cached_game.game;
            println!("Cached game: {:?}", game);
            channel.id().say(&http, &format!("🎮 Game of the Day: **{}**\n🔗 URL: https://store.steampowered.com/app/{}/", game.name, game.appid)).await?;
            return Ok(());
        }
    }

    let mut list_of_games: Vec<serde_json::Value> = Vec::new();
    let mut last_appid = 0;
    let mut file_number = 0;


    loop {

        let mut apps = read_json_file(&format!("{}/{}.json", STEAM_DATA_FOLDER, file_number)).await.unwrap_or_default();  // Read the cache from disk in a background thread
        if apps.is_null() {
            let url = format!("https://api.steampowered.com/IStoreService/GetAppList/v1/?key={}&format=json&max_results=50000&last_appid={}", steamapi_key, last_appid);  // # Replace {STEAM_KEY} with your actual Steam API key
            let response = fetch_data(&url).await;
            if let Some(response) = response {
                apps = response["response"].clone();
                let _ = write_json_file(&format!("{}/{}.json", STEAM_DATA_FOLDER, file_number), apps.clone()).await ;  // Save the cache to disk in a background thread
            }
        }


        list_of_games.extend(apps["apps"].as_array().unwrap_or(&Vec::new()).clone());
        last_appid = apps.get("last_appid").unwrap_or(&serde_json::Value::Number(serde_json::Number::from(0))).as_i64().unwrap_or(0);
        if !apps.get("have_more_results").unwrap_or(&serde_json::Value::Bool(false)).as_bool().unwrap_or(false) {
            break
        }

        file_number += 1;


    }
    
    if !list_of_games.is_empty() {
        let choosen_game = list_of_games.choose(&mut rand::rng()).unwrap();
        let game: GameInfo = serde_json::from_value(choosen_game.clone())?;
        channel.id().say(&http, &format!("🎮 Game of the Day: **{}**\n🔗 URL: https://store.steampowered.com/app/{}/", game.name, game.appid)).await?;
        println!("Selected game of the day: {:?}", game);

        let output_dict = serde_json::json!({
            guild_id.to_string() : {
                "game": game,
                "time": Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string()
            }
        });
        tokio::spawn(async move {
            let _ = write_json_file(&format!("{}/game_of_the_day.json", STEAM_DATA_FOLDER), output_dict).await;  // Save the cache to disk in a background thread
        });
    }
    Ok(())
}


pub fn start_daily_task(http: Arc<serenity::Http>) {
    tokio::spawn(async move {
        // target: 09:00:00 local time
        let target = NaiveTime::from_hms_opt(9, 0, 0).unwrap();

        loop {
            let now = Local::now();
            let today_target = now.date_naive().and_time(target);
            let today_target = Local.from_local_datetime(&today_target).unwrap();

            // if today's target already passed, aim for tomorrow
            let next = if today_target > now {
                today_target
            } else {
                today_target + ChronoDuration::days(1)
            };

            let wait = (next - now).to_std().unwrap_or(std::time::Duration::ZERO);
            tokio::time::sleep(wait).await;
            let channel = ChannelId::new(CHANNEL_ID).to_channel(&http).await.unwrap();
            if let Err(e) = game_of_day(&http, &channel).await {
                eprintln!("game_of_day failed: {e}");
            }
        }
    });
    println!("Started daily task for game of the day.");
}