mod session;
mod youtube;

use anyhow::Result;
use dotenv::dotenv;
use log::{info, warn, error};
use std::env;
use std::sync::Arc;
use teloxide::{
    dispatching::UpdateHandler,
    prelude::*,
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

use session::{SessionState, is_valid_youtube_url};

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
                msg.text().is_some() && msg.text().unwrap().contains("youtube") || 
                msg.text().is_some() && msg.text().unwrap().contains("youtu.be")
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
    if let Some(user_id) = msg.from().map(|user| user.id) {
        match cmd {
            Command::Help | Command::Start => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
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
                    bot.send_message(
                        msg.chat.id,
                        format!("You've joined session: {}", code)
                    ).await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "Invalid session code. Please check and try again."
                    ).await?;
                }
            }
            Command::Add(input) => {
                let mut state_guard = state.lock().await;
                
                if state_guard.is_in_session(&user_id) {
                    // Extract YouTube URL from input
                    let input_parts: Vec<&str> = input.split_whitespace().collect();
                    
                    if input_parts.is_empty() {
                        bot.send_message(
                            msg.chat.id,
                            "Please provide a YouTube URL with /add command."
                        ).await?;
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
                        match state_guard.add_to_queue(user_id, url, note) {
                            Ok(true) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "Added to queue! Type /queue to see current lineup."
                                ).await?;
                            },
                            Ok(false) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "This video is already in the queue."
                                ).await?;
                            },
                            Err(e) => {
                                error!("Error adding to queue: {}", e);
                                bot.send_message(
                                    msg.chat.id,
                                    "There was an error adding your video to the queue."
                                ).await?;
                            }
                        }
                    } else {
                        bot.send_message(
                            msg.chat.id,
                            "Please provide a valid YouTube URL."
                        ).await?;
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
                                "The queue is empty. Add videos with /add [youtube_url]"
                            ).await?;
                        } else {
                            let mut queue_text = "Current queue:\n".to_string();
                            
                            for (i, item) in queue_items.iter().enumerate() {
                                let note_text = match &item.note {
                                    Some(note) => format!(" - Note: {}", note),
                                    None => String::new(),
                                };
                                
                                queue_text.push_str(&format!(
                                    "{}. {} (added by User {}){}  \n", 
                                    i + 1, 
                                    item.video_info.url, 
                                    item.added_by.0,
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
                    bot.send_message(
                        msg.chat.id,
                        "You've left the session."
                    ).await?;
                } else {
                    bot.send_message(
                        msg.chat.id,
                        "You're not in a session."
                    ).await?;
                }
            }
        }
    } else {
        bot.send_message(msg.chat.id, "Sorry, I couldn't identify your user account.").await?;
    }

    Ok(())
}

// New function to handle messages containing YouTube URLs
async fn handle_youtube_message(
    bot: Bot,
    msg: Message,
    state: SharedState,
) -> ResponseResult<()> {
    if let (Some(text), Some(user_id)) = (msg.text(), msg.from().map(|user| user.id)) {
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
        if let Some(url_pos) = words.iter().position(|word| 
            word.contains("youtube.com") || word.contains("youtu.be")
        ) {
            let url = words[url_pos].to_string();
            
            // Everything before the URL goes into the note
            let before_note = if url_pos > 0 {
                Some(words[0..url_pos].join(" "))
            } else {
                None
            };
            
            // Everything after the URL goes into the note
            let after_note = if url_pos < words.len() - 1 {
                Some(words[url_pos+1..].join(" "))
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
                match state_guard.add_to_queue(user_id, url, note) {
                    Ok(true) => {
                        bot.send_message(
                            msg.chat.id,
                            "Added to queue! Type /queue to see current lineup."
                        ).await?;
                    },
                    Ok(false) => {
                        bot.send_message(
                            msg.chat.id,
                            "This video is already in the queue."
                        ).await?;
                    },
                    Err(e) => {
                        error!("Error adding to queue: {}", e);
                        bot.send_message(
                            msg.chat.id,
                            "There was an error adding your video to the queue."
                        ).await?;
                    }
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Please provide a valid YouTube URL."
                ).await?;
            }
        } else {
            // No URL found, do nothing
        }
    }
    
    Ok(())
}
