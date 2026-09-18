use rusty_ytdl::Video;
use rusty_ytdl::search::{YouTube, SearchResult, self, SearchOptions};
use std::collections::HashMap;
use rusty_ytdl::search::Playlist;
use tokio;
use crate::Error;
use crate::commands::music::Song;
use serde_json;
use std::process::Stdio;



pub async fn yt_query(query: &str) -> Result<Vec<Song>, Error> {
    if query.is_empty() {
        return Err("Query cannot be empty".into());
    }

    if query.contains("playlist"){

        let mut vec_result = Vec::new();

        let output = tokio::process::Command::new("yt-dlp")
            .args(["--flat-playlist", "-J", query])
            .output()
            .await?;

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let entries = json["entries"].as_array().unwrap().clone();

        let mut tasks = HashMap::new();

        for entry in entries {
            let url = entry["url"].as_str().unwrap_or("").to_string();
            let task = tokio::spawn(async move {
                Song {
                    title: entry["title"].as_str().unwrap_or("").to_string(),
                    url: entry["url"].as_str().unwrap_or("").to_string(),
                }
            });
            tasks.insert(url, task);
        }
        for (url, task) in tasks {
            if let Ok(song) = task.await {
                vec_result.push(song);
            }
        }

        println!("Found playlist with {} videos", vec_result.len());
        return Ok(vec_result);

    }
    else{
        let youtube = YouTube::new()?;


        let result = youtube.search_one(query, None).await?;

        if let Some(SearchResult::Video(video)) = result {

            return Ok(vec![Song {
                title: video.title.clone(),
                url: video.url.clone(),
            }]);
        }
    }

    
    Err("No video results found".into())


}

pub const DEFAULT_YT_VIDEO_OPTIONS: [&str; 16] = [
    "-o", "-",  
    "--format", "bestvideo[height<=360]+bestaudio/best",           // Pre-merged video/audio stream required for stdout/RAM
    "--quiet",
    "--no-warnings",
    "--socket-timeout", "15",
    "--retries", "10",
    "--fragment-retries", "10",
    "--skip-unavailable-fragments",
    "--no-playlist",
    "--merge-output-format", "mkv"
];

pub async fn yt_download(url: &str) -> Result<Vec<u8>, Error> {
    if url.is_empty() || !url.starts_with("http") {
        return Err("URL cannot be empty or invalid".into());
    }

    let output = tokio::process::Command::new("yt-dlp")
        .args(DEFAULT_YT_VIDEO_OPTIONS)
        .arg(url)
        .output()
        .await?;

    if !output.status.success() {
        eprintln!("yt-dlp failed with status: {}", output.status);
        eprintln!("yt-dlp stderr: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to download video".into());
    }

    Ok(output.stdout)
}