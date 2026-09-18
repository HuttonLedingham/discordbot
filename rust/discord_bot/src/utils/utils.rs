use songbird::input::{Input, RawAdapter};
use tokio::process::Command;
use std::process::Stdio;
use serde_json;
use std::collections::HashMap;

// async fn ytdl_ffmpeg_source(url: &str) -> Input {
//     // yt-dlp writes the audio to stdout; ffmpeg converts to 48kHz stereo s16le
//     let ytdl = Command::new("yt-dlp")
//         .args(["-f", "bestaudio", "-o", "-", url])
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .spawn()
//         .expect("yt-dlp failed to start");

//     let ffmpeg = Command::new("ffmpeg")
//         .args([
//             "-i", "pipe:0",
//             "-f", "s16le",
//             "-ar", "48000",
//             "-ac", "2",
//             "pipe:1",
//         ])
//         .stdin(Stdio::from(ytdl.stdout.unwrap().try_into().unwrap()))
//         .stdout(Stdio::piped())
//         .stderr(Stdio::null())
//         .spawn()
//         .expect("ffmpeg failed to start");

//     // RawAdapter wraps a reader of 48kHz stereo i16 PCM
//     RawAdapter::new(ffmpeg.stdout.unwrap(), 48000, 2).into()
// }

pub async fn read_hashmap_json_file(path: &str) -> Result<HashMap<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let data = tokio::fs::read_to_string(path).await?;
    let hashmap = serde_json::from_str::<HashMap<String, serde_json::Value>>(&data)?;
    Ok(hashmap)
}

pub async fn read_json_file(path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let data = tokio::fs::read_to_string(path).await?;
    let json = serde_json::from_str::<serde_json::Value>(&data)?;
    Ok(json)
}

pub async fn write_json_file(path: &str, data: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(&data)?;
    tokio::fs::write(path, json).await?;
    Ok(())
}