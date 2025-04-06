use regex::Regex;
use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use serde::Deserialize;

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

pub fn validate_youtube_url(url: &str) -> bool {
    YOUTUBE_URL_REGEX.is_match(url)
}

pub fn extract_video_id(url: &str) -> Option<String> {
    YOUTUBE_URL_REGEX.captures(url).and_then(|cap| {
        cap.get(6).map(|m| m.as_str().to_string())
    })
}

pub fn create_video_info(url: &str) -> Result<VideoInfo> {
    let video_id = extract_video_id(url)
        .ok_or_else(|| anyhow!("Failed to extract video ID from URL"))?;
    
    Ok(VideoInfo {
        id: video_id,
        title: None, // Future enhancement: Fetch title from YouTube API
        url: url.to_string(),
    })
}

// Function to get embed URL for a video
pub fn get_embed_url(video_id: &str) -> String {
    format!("https://www.youtube.com/embed/{}", video_id)
}

// Simple check if it's a duplicate URL by comparing video IDs
pub fn is_duplicate(url1: &str, url2: &str) -> bool {
    match (extract_video_id(url1), extract_video_id(url2)) {
        (Some(id1), Some(id2)) => id1 == id2,
        _ => false,
    }
} 