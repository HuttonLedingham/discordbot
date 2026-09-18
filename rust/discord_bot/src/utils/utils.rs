use serde_json;
use std::collections::HashMap;

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