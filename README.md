# Karaoke Queue Bot

This Telegram bot allows users to join a karaoke session and add YouTube videos to a shared queue for playback.

## Features

- Create and join karaoke sessions with unique session codes
- Add YouTube videos to a shared queue
- View the current queue
- Automatic validation of YouTube links
- Cast videos to a Chromecast/TV (simulated)
- Track currently playing video and history
- Session persistence across bot restarts (coming soon)
- Queue prioritization for users who haven't gone in a while (coming soon)

## Getting Started

### Prerequisites

- Rust and Cargo installed
- Telegram Bot token (obtainable from @BotFather)
- YouTube API key (obtainable from Google Cloud Console)

### Setup

1. Clone the repository
2. Create a `.env` file in the root directory with:
```
TELEGRAM_BOT_TOKEN=your_telegram_bot_token_here
YOUTUBE_API_KEY=your_youtube_api_key_here
RUST_LOG=info
```
3. Build the project:
```
cargo build
```
4. Run the bot:
```
cargo run
```

### Getting a YouTube API Key

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project
3. Enable the YouTube Data API v3
4. Create credentials (API key)
5. Copy the API key to your `.env` file

## Bot Commands

- `/help`: Display help information
- `/start`: Display help information
- `/start-session`: Create a new karaoke session
- `/join [code]`: Join an existing session with a code
- `/add [youtube_url]`: Add a YouTube link to the queue
- `/queue`: View current queue
- `/leave`: Leave current session
- `/next`: Play the next video in the queue (session owner only)
- `/current`: Display the video playing now
- `/history`: View all videos previously played

## Casting Functionality

The bot includes a simulated casting functionality. When the session owner uses the `/next` command, the bot:

1. Pops the next video from the queue
2. Updates the current playing video
3. Simulates casting to a device (currently a placeholder for real implementation)
4. Tracks the video in history

In a real implementation, this would connect to a Chromecast or other casting device to actually play the video.

## Future Enhancements

- [x] a message containing a youtube link should automatically be added to the queue
- [x] Display video titles and usernames in queue
- [x] Casting functionality to a Chromecast/TV (simulated)
- [x] `/current` to display the video playing now
- [x] `/history` to see all videos previously played
- [ ] Implement actual connection to Chromecast devices
- [ ] Persistent storage for sessions and queue items
- [ ] Prioritize queue so users who haven't gone in a while get queued up sooner
- [ ] Admin controls for managing sessions
- [ ] Support for other video platforms
- [ ] Send message to user when their video is next in line

See [ORIGINAL_REQUIREMENTS.md](ORIGINAL_REQUIREMENTS.md) for the initial project requirements and design notes.


