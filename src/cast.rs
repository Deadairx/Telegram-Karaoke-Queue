use anyhow::{anyhow, Result};
use log::{info, warn, error, debug};
use regex::Regex;
use rust_cast::channels::media;
use rust_cast::CastDevice;
use std::collections::HashMap;
use std::{process::Command, sync::Arc};
use tokio::sync::Mutex;
use tokio::time::timeout;
use std::time::Duration;

use crate::youtube::VideoInfo;

// Cast status for a session
#[derive(Debug, Clone)]
pub struct CastStatus {
    pub current_video: Option<VideoInfo>,
    pub cast_device: Option<String>,
    pub is_playing: bool,
}

impl Default for CastStatus {
    fn default() -> Self {
        Self {
            current_video: None,
            cast_device: None,
            is_playing: false,
        }
    }
}

// Store active connections to cast devices
type CastConnections = Arc<Mutex<HashMap<String, String>>>; // Store host:port instead of CastDevice
lazy_static::lazy_static! {
    static ref CAST_CONNECTIONS: CastConnections = Arc::new(Mutex::new(HashMap::new()));
    static ref DEVICE_REGEX: Regex = Regex::new(r"([^\s]+)\s+_\googlecast\._tcp\.\s+local\.").unwrap();
}

#[derive(Debug)]
struct ChromecastDevice {
    name: String,
    host: String,
    port: u16,
}

// Discover Chromecast devices using dns-sd
async fn discover_chromecasts() -> Result<Vec<ChromecastDevice>> {
    info!("Starting Chromecast device discovery...");
    
    // Run dns-sd command with a 10 second timeout
    // Using -G for one-time lookup instead of -B for continuous browsing
    let output = match timeout(
        Duration::from_secs(10),
        tokio::process::Command::new("dns-sd")
            .args(["-G", "v4", "_googlecast._tcp", "local"])
            .output()
    ).await {
        Ok(Ok(output)) => {
            debug!("dns-sd command output: {}", String::from_utf8_lossy(&output.stdout));
            if !output.stderr.is_empty() {
                warn!("dns-sd stderr: {}", String::from_utf8_lossy(&output.stderr));
            }
            output
        },
        Ok(Err(e)) => {
            error!("Failed to run dns-sd command: {}", e);
            return Err(anyhow!("Failed to run dns-sd: {}", e));
        },
        Err(_) => {
            error!("dns-sd command timed out after 10 seconds");
            return Err(anyhow!("dns-sd command timed out after 10 seconds"));
        },
    };

    if !output.status.success() {
        error!("dns-sd command failed with status: {}", output.status);
        return Err(anyhow!("dns-sd command failed with status: {}", output.status));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    debug!("Raw dns-sd output: {}", output_str);
    
    let mut devices = Vec::new();

    // Parse each line that contains a device
    for line in output_str.lines() {
        debug!("Processing line: {}", line);
        
        // Updated regex to match the -G output format
        if line.contains("_googlecast._tcp.local.") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let name = parts[0];
                let host = parts[4].trim_end_matches('.');
                
                info!("Found Chromecast device: {} at {}", name, host);
                
                // Default port for Chromecast is 8009
                devices.push(ChromecastDevice {
                    name: name.to_string(),
                    host: host.to_string(),
                    port: 8009,
                });
            } else {
                warn!("Unexpected line format: {}", line);
            }
        }
    }

    if devices.is_empty() {
        warn!("No Chromecast devices found on the network");
    } else {
        info!("Found {} Chromecast devices", devices.len());
    }

    Ok(devices)
}

// This function actually sends the video to a cast device
pub async fn cast_video(video_info: &VideoInfo, device_name: Option<&str>) -> Result<bool> {
    // Get the embed URL for the video
    let embed_url = crate::youtube::get_embed_url(&video_info.id);

    // Log casting attempt
    let device = match device_name {
        Some(name) => name.to_string(),
        None => {
            // If no device specified, try to use the first available device
            let devices = get_available_devices().await?;
            if devices.is_empty() {
                return Err(anyhow!("No cast devices available"));
            }
            devices[0].clone()
        }
    };

    info!("Casting video {} to {}", video_info.id, device);

    // Check if we already have a connection to this device
    let mut connections = CAST_CONNECTIONS.lock().await;

    // Get or create connection to the device
    let device_key = if let Some(existing_key) = connections.get(&device) {
        existing_key.clone()
    } else {
        // Try to find and connect to the device
        let devices = discover_chromecasts().await?;

        let found_device = devices
            .iter()
            .find(|d| d.name == device)
            .ok_or_else(|| anyhow!("Cast device not found: {}", device))?;

        let device_key = format!("{}:{}", found_device.host, found_device.port);
        connections.insert(device.clone(), device_key.clone());
        device_key
    };

    // Create a new connection for this request
    let cast_device = CastDevice::connect(
        device_key.split(':').next().unwrap(),
        device_key.split(':').nth(1).unwrap().parse()?,
    )?;

    // Get video info for media
    let title = video_info
        .title
        .clone()
        .unwrap_or_else(|| "YouTube Video".to_string());

    // Prepare media content
    let content_id = format!("https://www.youtube.com/watch?v={}", video_info.id);

    // Create media object - simplified version for YouTube
    let metadata = media::Metadata::Generic(media::GenericMediaMetadata {
        title: Some(title),
        subtitle: None,
        images: Vec::new(),
        release_date: None,
    });

    let media_info = media::Media {
        content_id,
        content_type: "application/x-youtube".to_string(),
        stream_type: media::StreamType::Buffered,
        duration: None,
        metadata: Some(metadata),
    };

    // Load media
    cast_device.media.load("receiver-0", "1", &media_info)?;

    // Return success
    Ok(true)
}

// Get a list of available cast devices
pub async fn get_available_devices() -> Result<Vec<String>> {
    // Discover devices on the network
    let devices = discover_chromecasts().await?;

    // Return device names
    Ok(devices.iter().map(|d| d.name.clone()).collect())
}

// Stop any currently playing video
pub async fn stop_casting(device_name: Option<&str>) -> Result<bool> {
    let device = match device_name {
        Some(name) => name.to_string(),
        None => {
            // If no device, try to stop on all connected devices
            let connections = CAST_CONNECTIONS.lock().await;
            if connections.is_empty() {
                return Err(anyhow!("No connected cast devices"));
            }

            // Use the first connected device
            connections
                .keys()
                .next()
                .ok_or_else(|| anyhow!("No connected devices"))?
                .clone()
        }
    };

    info!("Stopping casting on {}", device);

    // Get connection to device
    let connections = CAST_CONNECTIONS.lock().await;
    let device_key = connections
        .get(&device)
        .ok_or_else(|| anyhow!("Not connected to device: {}", device))?;

    // Create a new connection for this request
    let cast_device = CastDevice::connect(
        device_key.split(':').next().unwrap(),
        device_key.split(':').nth(1).unwrap().parse()?,
    )?;

    // Send stop request
    cast_device.media.stop("receiver-0", 1)?;

    Ok(true)
}
