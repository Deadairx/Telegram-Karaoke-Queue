use anyhow::Result;
use chrono;
use rand::Rng;
use std::collections::HashMap;
use teloxide::types::UserId;

use crate::cast::CastStatus;
use crate::youtube::{create_video_info, validate_youtube_url, VideoInfo};

#[derive(Clone, Default)]
pub struct SessionState {
    pub sessions: HashMap<String, Session>,
    pub user_sessions: HashMap<UserId, String>, // Maps Telegram UserId to session code
}

#[derive(Clone)]
pub struct Session {
    pub code: String,
    pub users: Vec<UserId>,
    pub queue: Vec<QueueItem>,
    pub owner: UserId,           // Track who created the session
    pub cast_status: CastStatus, // Track current casting status
}

#[derive(Clone)]
pub struct QueueItem {
    pub video_info: VideoInfo,
    pub added_by: UserId,
    pub username: Option<String>, // Store username of the person who added this item
    pub added_at: i64,
    pub played: bool,
    pub note: Option<String>, // Optional note for the queue item
}

impl SessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_session(&mut self, user_id: UserId) -> String {
        let session_code = generate_session_code();

        let new_session = Session {
            code: session_code.clone(),
            users: vec![user_id],
            queue: Vec::new(),
            owner: user_id,
            cast_status: CastStatus::default(),
        };

        self.sessions.insert(session_code.clone(), new_session);
        self.user_sessions.insert(user_id, session_code.clone());

        session_code
    }

    pub fn join_session(&mut self, user_id: UserId, code: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(code) {
            // Add user to session if not already in it
            if !session.users.contains(&user_id) {
                session.users.push(user_id);
            }
            self.user_sessions.insert(user_id, code.to_string());
            true
        } else {
            false
        }
    }

    pub async fn add_to_queue(
        &mut self,
        user_id: UserId,
        url: String,
        username: Option<String>,
        note: Option<String>,
    ) -> Result<bool> {
        let session_code = self
            .user_sessions
            .get(&user_id)
            .ok_or_else(|| anyhow::anyhow!("User not in a session"))?;

        let session = self
            .sessions
            .get_mut(session_code)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Remove duplicate check
        let video_info = create_video_info(&url).await?;

        let queue_item = QueueItem {
            video_info,
            added_by: user_id,
            username,
            added_at: chrono::Utc::now().timestamp(),
            played: false,
            note,
        };

        session.queue.push(queue_item);
        Ok(true)
    }

    pub fn get_queue(&self, user_id: &UserId) -> Option<Vec<&QueueItem>> {
        let session_code = self.user_sessions.get(user_id)?;
        let session = self.sessions.get(session_code)?;

        let items = session.queue.iter().filter(|item| !item.played).collect();

        Some(items)
    }

    pub fn leave_session(&mut self, user_id: &UserId) -> bool {
        if let Some(session_code) = self.user_sessions.remove(user_id) {
            if let Some(session) = self.sessions.get_mut(&session_code) {
                // Remove user from session
                session.users.retain(|id| *id != *user_id);

                // If session is empty, remove it
                if session.users.is_empty() {
                    self.sessions.remove(&session_code);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn is_in_session(&self, user_id: &UserId) -> bool {
        self.user_sessions.contains_key(user_id)
    }

    // Check if user is the session owner
    pub fn is_session_owner(&self, user_id: &UserId) -> bool {
        if let Some(session_code) = self.user_sessions.get(user_id) {
            if let Some(session) = self.sessions.get(session_code) {
                return session.owner == *user_id;
            }
        }
        false
    }

    // Get the next item in the queue and mark it as current
    pub fn next_in_queue(&mut self, user_id: &UserId) -> Option<QueueItem> {
        // Only allow session owner to advance the queue
        if !self.is_session_owner(user_id) {
            return None;
        }

        let session_code = self.user_sessions.get(user_id)?;
        let session = self.sessions.get_mut(session_code)?;

        // Find the first unplayed item
        let next_item_index = session.queue.iter().position(|item| !item.played);

        if let Some(index) = next_item_index {
            // Mark item as played
            session.queue[index].played = true;

            // Set current video in cast status
            session.cast_status.current_video = Some(session.queue[index].video_info.clone());

            // Return a clone of the item
            return Some(session.queue[index].clone());
        }

        None
    }

    // Get the current playing video
    pub fn get_current_video(&self, user_id: &UserId) -> Option<&VideoInfo> {
        let session_code = self.user_sessions.get(user_id)?;
        let session = self.sessions.get(session_code)?;

        session.cast_status.current_video.as_ref()
    }

    // Get history of played videos
    pub fn get_history(&self, user_id: &UserId) -> Option<Vec<&QueueItem>> {
        let session_code = self.user_sessions.get(user_id)?;
        let session = self.sessions.get(session_code)?;

        let items = session.queue.iter().filter(|item| item.played).collect();

        Some(items)
    }

    // Set the device for a session
    pub fn set_device(&mut self, user_id: &UserId, device_name: &str) -> bool {
        if let Some(session_code) = self.user_sessions.get(user_id) {
            if let Some(session) = self.sessions.get_mut(session_code) {
                session.cast_status.cast_device = Some(device_name.to_string());
                return true;
            }
        }
        false
    }

    // Get the current device for a session
    pub fn get_device(&self, user_id: &UserId) -> Option<String> {
        if let Some(session_code) = self.user_sessions.get(user_id) {
            if let Some(session) = self.sessions.get(session_code) {
                return session.cast_status.cast_device.clone();
            }
        }
        None
    }
}

// Generate a random 6-character session code
pub fn generate_session_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// Public function to validate YouTube URL
pub fn is_valid_youtube_url(url: &str) -> bool {
    validate_youtube_url(url)
}
