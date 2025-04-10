use anyhow::Result;
use chrono;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use teloxide::types::UserId;

use crate::cast::CastStatus;
use crate::youtube::{create_video_info, validate_youtube_url, VideoInfo};

const SESSION_FILE: &str = "sessions.json";

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub sessions: HashMap<String, Session>,
    pub user_sessions: HashMap<UserId, String>, // Maps Telegram UserId to session code
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub code: String,
    pub users: Vec<(UserId, Option<String>)>, // (user_id, username)
    pub queue: Vec<QueueItem>,
    pub owner: UserId,           // Track who created the session
    pub cast_status: CastStatus, // Track current casting status
    pub created_at: i64,         // Unix timestamp when session was created
}

#[derive(Clone, Serialize, Deserialize)]
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
        Self::load().unwrap_or_else(|_| Self::default())
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(SESSION_FILE, json)?;
        Ok(())
    }

    pub fn load() -> Result<Self> {
        if Path::new(SESSION_FILE).exists() {
            let json = fs::read_to_string(SESSION_FILE)?;
            let state: SessionState = serde_json::from_str(&json)?;
            Ok(state)
        } else {
            Ok(SessionState::default())
        }
    }

    pub fn create_session(&mut self, user_id: UserId, username: Option<String>) -> String {
        let session_code = generate_session_code();

        let new_session = Session {
            code: session_code.clone(),
            users: vec![(user_id, username)],
            queue: Vec::new(),
            owner: user_id,
            cast_status: CastStatus::default(),
            created_at: chrono::Utc::now().timestamp(),
        };

        self.sessions.insert(session_code.clone(), new_session);
        self.user_sessions.insert(user_id, session_code.clone());

        // Save state after creating session
        if let Err(e) = self.save() {
            eprintln!("Failed to save session state: {}", e);
        }

        session_code
    }

    pub fn join_session(&mut self, user_id: UserId, username: Option<String>, code: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(code) {
            // Add user to session if not already in it
            if !session.users.iter().any(|(id, _)| *id == user_id) {
                session.users.push((user_id, username));
            }
            self.user_sessions.insert(user_id, code.to_string());

            // Save state after joining session
            if let Err(e) = self.save() {
                eprintln!("Failed to save session state: {}", e);
            }

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

        // Save state after adding to queue
        if let Err(e) = self.save() {
            eprintln!("Failed to save session state: {}", e);
        }

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
                session.users.retain(|(id, _)| *id != *user_id);

                // If session is empty, remove it
                if session.users.is_empty() {
                    self.sessions.remove(&session_code);
                }
            }

            // Save state after leaving session
            if let Err(e) = self.save() {
                eprintln!("Failed to save session state: {}", e);
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

        // First, find the next unplayed item and clone it
        let next_item = {
            let session = self.sessions.get(session_code)?;
            let next_item_index = session.queue.iter().position(|item| !item.played)?;
            session.queue[next_item_index].clone()
        };

        // Then, update the session state
        if let Some(session) = self.sessions.get_mut(session_code) {
            if let Some(index) = session.queue.iter().position(|item| !item.played) {
                // Mark item as played
                session.queue[index].played = true;

                // Set current video in cast status
                session.cast_status.current_video = Some(session.queue[index].video_info.clone());

                // Save state after advancing queue
                if let Err(e) = self.save() {
                    eprintln!("Failed to save session state: {}", e);
                }
            }
        }

        Some(next_item)
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

    pub fn get_session_info(&self, user_id: &UserId) -> Option<String> {
        let session_code = self.user_sessions.get(user_id)?;
        let session = self.sessions.get(session_code)?;

        let duration = chrono::Utc::now().timestamp() - session.created_at;
        let hours = duration / 3600;
        let minutes = (duration % 3600) / 60;

        let mut info = format!(
            "Session ID: {}\nDuration: {}h {}m\nUsers in session: {}",
            session.code,
            hours,
            minutes,
            session.users.len()
        );

        // If user is the owner, add list of users
        if session.owner == *user_id {
            info.push_str("\n\nUsers in session:");
            for (_, username) in &session.users {
                let user_display = username.clone().unwrap_or_else(|| "Anonymous".to_string());
                info.push_str(&format!("\n- {}", user_display));
            }
        }

        Some(info)
    }
}

// Generate a random 4-digit session code
pub fn generate_session_code() -> String {
    let mut rng = rand::thread_rng();
    format!("{:04}", rng.gen_range(0..10000))
}

// Public function to validate YouTube URL
pub fn is_valid_youtube_url(url: &str) -> bool {
    validate_youtube_url(url)
}
