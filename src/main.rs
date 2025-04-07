mod session;
mod youtube;
mod cast;

use anyhow::Result;
use dotenv::dotenv;
use log::{error, info, warn};
use std::env;
use std::sync::Arc;
use teloxide::{dispatching::UpdateHandler, prelude::*, utils::command::BotCommands};
use tokio::sync::Mutex;

use session::{is_valid_youtube_url, SessionState};
use cast::cast_video;

// Bot commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Display this help message")]
    Help,
    #[command(description = "Display help information")]
    Start,
    #[command(description = "Start a new karaoke session")]
    StartSession,
    #[command(description = "Join an existing session with code")]
    Join(String),
    #[command(description = "Add a YouTube link to the queue (with optional note)")]
    Add(String),
    #[command(description = "View current queue")]
    Queue,
    #[command(description = "Leave current session")]
    Leave,
    #[command(description = "Play the next video in the queue (session owner only)")]
    Next,
    #[command(description = "Display the currently playing video")]
    Current,
    #[command(description = "View history of played videos")]
    History,
}

// State shared between command handlers
type SharedState = Arc<Mutex<SessionState>>;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    pretty_env_logger::init();
    info!("Starting karaoke queue bot");

    let bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN must be set"))?;
    let bot = Bot::new(bot_token);

    let state = Arc::new(Mutex::new(SessionState::new()));

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(handle_command),
        )
        .branch(
            dptree::filter(|msg: Message| {
                msg.text().is_some() && msg.text().unwrap().contains("youtube")
                    || msg.text().is_some() && msg.text().unwrap().contains("youtu.be")
            })
            .endpoint(handle_youtube_message),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: SharedState,
) -> ResponseResult<()> {
    if let Some(user) = msg.from() {
        let user_id = user.id;
        let username = user.username.clone().or_else(|| {
            Some(
                format!(
                    "{} {}",
                    user.first_name.clone(),
                    user.last_name.clone().unwrap_or_default()
                )
                .trim()
                .to_string(),
            )
        });

        match cmd {
            Command::Help | Command::Start => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Command::StartSession => {
                let mut state_guard = state.lock().await;
                let session_code = state_guard.create_session(user_id);

                bot.send_message(
                    msg.chat.id,
                    format!("Created new karaoke session with code: {}\nShare this code with friends to let them join!", session_code)
                ).await?;
            }
            Command::Join(code) => {
                let code = code.trim();
                let mut state_guard = state.lock().await;

                if state_guard.join_session(user_id, code) {
                    bot.send_message(msg.chat.id, format!("You've joined session: {}", code))
                        .await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "Invalid session code. Please check and try again.",
                    )
                    .await?;
                }
            }
            Command::Add(input) => {
                let input_cloned = input.clone();
                let mut state_guard = state.lock().await;

                if state_guard.is_in_session(&user_id) {
                    // Extract YouTube URL from input
                    let input_parts: Vec<&str> = input_cloned.split_whitespace().collect();

                    if input_parts.is_empty() {
                        bot.send_message(
                            msg.chat.id,
                            "Please provide a YouTube URL with /add command.",
                        )
                        .await?;
                        return Ok(());
                    }

                    let url = input_parts[0].to_string();

                    // Get the rest of the input as note (if any)
                    let note = if input_parts.len() > 1 {
                        Some(input_parts[1..].join(" "))
                    } else {
                        None
                    };

                    if is_valid_youtube_url(&url) {
                        match state_guard.add_to_queue(user_id, url, username, note).await {
                            Ok(true) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "Added to queue! Type /queue to see current lineup.",
                                )
                                .await?;
                            }
                            Ok(false) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "This video is already in the queue.",
                                )
                                .await?;
                            }
                            Err(e) => {
                                error!("Error adding to queue: {}", e);
                                bot.send_message(
                                    msg.chat.id,
                                    "There was an error adding your video to the queue.",
                                )
                                .await?;
                            }
                        }
                    } else {
                        bot.send_message(msg.chat.id, "Please provide a valid YouTube URL.")
                            .await?;
                    }
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "You're not in a session. Join one with /join [code] or start your own with /start-session"
                    ).await?;
                }
            }
            Command::Queue => {
                let state_guard = state.lock().await;

                if state_guard.is_in_session(&user_id) {
                    if let Some(queue_items) = state_guard.get_queue(&user_id) {
                        if queue_items.is_empty() {
                            bot.send_message(
                                msg.chat.id,
                                "The queue is empty. Add videos with /add [youtube_url]",
                            )
                            .await?;
                        } else {
                            let mut queue_text = "Current queue:\n".to_string();

                            for (i, item) in queue_items.iter().enumerate() {
                                let note_text = match &item.note {
                                    Some(note) => format!(" - Note: {}", note),
                                    None => String::new(),
                                };

                                // Get video title or use ID if title is not available
                                let video_name = match &item.video_info.title {
                                    Some(title) => title.clone(),
                                    None => format!("Video ID: {}", item.video_info.id),
                                };

                                // Get the username or use a default
                                let user_identifier = match &item.username {
                                    Some(name) => name.clone(),
                                    None => format!("User {}", item.added_by.0),
                                };

                                queue_text.push_str(&format!(
                                    "{}. {} (added by {}){}  \n",
                                    i + 1,
                                    video_name,
                                    user_identifier,
                                    note_text
                                ));
                            }

                            bot.send_message(msg.chat.id, queue_text).await?;
                        }
                    }
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "You're not in a session. Join one with /join [code] or start your own with /start-session"
                    ).await?;
                }
            }
            Command::Leave => {
                let mut state_guard = state.lock().await;

                if state_guard.leave_session(&user_id) {
                    bot.send_message(msg.chat.id, "You've left the session.")
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, "You're not in a session.")
                        .await?;
                }
            }
            Command::Next => {
                let mut state_guard = state.lock().await;
                
                if !state_guard.is_in_session(&user_id) {
                    bot.send_message(
                        msg.chat.id, 
                        "You're not in a session. Join one with /join [code] or start your own with /start-session"
                    ).await?;
                    return Ok(());
                }
                
                if !state_guard.is_session_owner(&user_id) {
                    bot.send_message(
                        msg.chat.id,
                        "Only the session owner can advance the queue."
                    ).await?;
                    return Ok(());
                }
                
                match state_guard.next_in_queue(&user_id) {
                    Some(next_item) => {
                        // Get video title
                        let video_title = next_item.video_info.title.clone()
                            .unwrap_or_else(|| format!("Video ID: {}", next_item.video_info.id));
                        
                        // Get username
                        let user_name = next_item.username.clone()
                            .unwrap_or_else(|| format!("User {}", next_item.added_by.0));
                        
                        // Try to cast the video
                        let video_info = next_item.video_info.clone();
                        
                        // Drop the mutex guard before the next await point to avoid deadlocks
                        drop(state_guard);
                        
                        // Try to cast the video
                        match cast_video(&video_info, None).await {
                            Ok(_) => {
                                bot.send_message(
                                    msg.chat.id,
                                    format!("Now playing: {} (added by {})", video_title, user_name)
                                ).await?;
                            }
                            Err(e) => {
                                error!("Error casting video: {}", e);
                                bot.send_message(
                                    msg.chat.id,
                                    format!("Error casting video: {}", e)
                                ).await?;
                            }
                        }
                    }
                    None => {
                        bot.send_message(
                            msg.chat.id,
                            "No more videos in the queue. Add videos with /add [youtube_url]"
                        ).await?;
                    }
                }
            }
            Command::Current => {
                let state_guard = state.lock().await;
                
                if !state_guard.is_in_session(&user_id) {
                    bot.send_message(
                        msg.chat.id,
                        "You're not in a session. Join one with /join [code] or start your own with /start-session"
                    ).await?;
                    return Ok(());
                }
                
                match state_guard.get_current_video(&user_id) {
                    Some(video) => {
                        let video_title = video.title.clone()
                            .unwrap_or_else(|| format!("Video ID: {}", video.id));
                        
                        bot.send_message(
                            msg.chat.id,
                            format!("Currently playing: {}\nLink: {}", video_title, video.url)
                        ).await?;
                    }
                    None => {
                        bot.send_message(
                            msg.chat.id,
                            "No video is currently playing. Use /next to play the next video in queue."
                        ).await?;
                    }
                }
            }
            Command::History => {
                let state_guard = state.lock().await;
                
                if !state_guard.is_in_session(&user_id) {
                    bot.send_message(
                        msg.chat.id,
                        "You're not in a session. Join one with /join [code] or start your own with /start-session"
                    ).await?;
                    return Ok(());
                }
                
                match state_guard.get_history(&user_id) {
                    Some(history_items) if !history_items.is_empty() => {
                        let mut history_text = "Previously played videos:\n".to_string();
                        
                        for (i, item) in history_items.iter().enumerate() {
                            let video_title = item.video_info.title.clone()
                                .unwrap_or_else(|| format!("Video ID: {}", item.video_info.id));
                            
                            let user_name = item.username.clone()
                                .unwrap_or_else(|| format!("User {}", item.added_by.0));
                            
                            history_text.push_str(&format!(
                                "{}. {} (added by {})\n",
                                i + 1,
                                video_title,
                                user_name
                            ));
                        }
                        
                        bot.send_message(msg.chat.id, history_text).await?;
                    }
                    _ => {
                        bot.send_message(
                            msg.chat.id,
                            "No videos have been played yet in this session."
                        ).await?;
                    }
                }
            }
        }
    } else {
        bot.send_message(msg.chat.id, "Sorry, I couldn't identify your user account.")
            .await?;
    }

    Ok(())
}

// New function to handle messages containing YouTube URLs
async fn handle_youtube_message(bot: Bot, msg: Message, state: SharedState) -> ResponseResult<()> {
    if let (Some(text), Some(user)) = (msg.text(), msg.from()) {
        let user_id = user.id;
        let username = user.username.clone().or_else(|| {
            Some(
                format!(
                    "{} {}",
                    user.first_name.clone(),
                    user.last_name.clone().unwrap_or_default()
                )
                .trim()
                .to_string(),
            )
        });

        let mut state_guard = state.lock().await;

        if !state_guard.is_in_session(&user_id) {
            bot.send_message(
                msg.chat.id,
                "You're not in a session. Join one with /join [code] or start your own with /start-session"
            ).await?;
            return Ok(());
        }

        // Extract YouTube URL and note
        let words: Vec<&str> = text.split_whitespace().collect();

        // Find the first YouTube URL in the message
        if let Some(url_pos) = words
            .iter()
            .position(|word| word.contains("youtube.com") || word.contains("youtu.be"))
        {
            let url = words[url_pos].to_string();

            // Everything before the URL goes into the note
            let before_note = if url_pos > 0 {
                Some(words[0..url_pos].join(" "))
            } else {
                None
            };

            // Everything after the URL goes into the note
            let after_note = if url_pos < words.len() - 1 {
                Some(words[url_pos + 1..].join(" "))
            } else {
                None
            };

            // Combine notes if needed
            let note = match (before_note, after_note) {
                (Some(before), Some(after)) => Some(format!("{} {}", before, after)),
                (Some(note), None) | (None, Some(note)) => Some(note),
                (None, None) => None,
            };

            if is_valid_youtube_url(&url) {
                match state_guard.add_to_queue(user_id, url, username, note).await {
                    Ok(true) => {
                        bot.send_message(
                            msg.chat.id,
                            "Added to queue! Type /queue to see current lineup.",
                        )
                        .await?;
                    }
                    Ok(false) => {
                        bot.send_message(msg.chat.id, "This video is already in the queue.")
                            .await?;
                    }
                    Err(e) => {
                        error!("Error adding to queue: {}", e);
                        bot.send_message(
                            msg.chat.id,
                            "There was an error adding your video to the queue.",
                        )
                        .await?;
                    }
                }
            } else {
                bot.send_message(msg.chat.id, "Please provide a valid YouTube URL.")
                    .await?;
            }
        } else {
            // No URL found, do nothing
        }
    }

    Ok(())
}
