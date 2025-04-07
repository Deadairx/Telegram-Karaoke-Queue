use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;

lazy_static! {
    static ref YOUTUBE_URL_REGEX: Regex = Regex::new(
        r"^((?:https?:)?//)?((?:www|m)\.)?((?:youtube(-nocookie)?\.com|youtu.be))(/(?:[\w\-]+\?v=|embed/|v/)?)([\w\-]+)(\S+)?$"
    ).expect("Invalid YouTube URL regex pattern");
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: Option<String>,
    pub url: String,
}

// YouTube API response structures
#[derive(Debug, Deserialize)]
struct YouTubeResponse {
    items: Vec<YouTubeItem>,
}

#[derive(Debug, Deserialize)]
struct YouTubeItem {
    snippet: YouTubeSnippet,
}

#[derive(Debug, Deserialize)]
struct YouTubeSnippet {
    title: String,
}

pub fn validate_youtube_url(url: &str) -> bool {
    YOUTUBE_URL_REGEX.is_match(url)
}

pub fn extract_video_id(url: &str) -> Option<String> {
    YOUTUBE_URL_REGEX
        .captures(url)
        .and_then(|cap| cap.get(6).map(|m| m.as_str().to_string()))
}

pub async fn create_video_info(url: &str) -> Result<VideoInfo> {
    let video_id =
        extract_video_id(url).ok_or_else(|| anyhow!("Failed to extract video ID from URL"))?;

    // Try to fetch title from YouTube API, but fall back gracefully
    let title = match fetch_video_title(&video_id).await {
        Ok(Some(title)) => Some(title),
        Ok(None) => Some(format!("YouTube Video: {}", video_id)),
        Err(e) => {
            // Log the error but don't fail the whole operation
            log::warn!("Failed to fetch video title: {}", e);
            Some(format!("YouTube Video: {}", video_id))
        }
    };

    Ok(VideoInfo {
        id: video_id.clone(),
        title,
        url: url.to_string(),
    })
}

async fn fetch_video_title(video_id: &str) -> Result<Option<String>> {
    // Get API key from environment, but don't fail if not present
    let api_key = match env::var("YOUTUBE_API_KEY") {
        Ok(key) => key,
        Err(_) => return Err(anyhow!("YOUTUBE_API_KEY not set")),
    };

    // Build the API URL
    let api_url = format!(
        "https://www.googleapis.com/youtube/v3/videos?id={}&key={}&part=snippet",
        video_id, api_key
    );

    // Make the API request
    let client = reqwest::Client::new();
    let response = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| anyhow!("YouTube API request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!("YouTube API returned error: {}", response.status()));
    }

    let youtube_data: YouTubeResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse YouTube API response: {}", e))?;

    // Extract the title
    if youtube_data.items.is_empty() {
        // Video not found or API error
        Ok(None)
    } else {
        let title = youtube_data.items[0].snippet.title.clone();
        Ok(Some(title))
    }
}

// Function to get embed URL for a video
pub fn get_embed_url(video_id: &str) -> String {
    format!("https://www.youtube.com/embed/{}", video_id)
}
