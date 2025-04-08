# Karaoke Queue Bot

This Telegram bot allows users to join a karaoke session and add YouTube videos to a shared queue for playback on a Chromecast device. This implementation is specifically designed for macOS.

## Features

- Create and join karaoke sessions with unique session codes
- Add YouTube videos to a shared queue
- View the current queue
- Automatic validation of YouTube links
- Cast videos to Chromecast devices on your network (macOS only)
- Track currently playing video and history
- Session persistence across bot restarts (coming soon)
- Queue prioritization for users who haven't gone in a while (coming soon)

## Getting Started

### Prerequisites

- macOS (required for Chromecast discovery)
- Rust and Cargo installed
- Telegram Bot token (obtainable from @BotFather)
- YouTube API key (obtainable from Google Cloud Console)
- A Chromecast device on your local network

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
- `/devices`: List all available Chromecast devices on your network
- `/set-device [device name]`: Set which Chromecast device to use (session owner only)

## Casting Functionality

The bot includes Chromecast integration using the `rust_cast` crate and macOS's `dns-sd` command for device discovery. When the session owner uses the `/next` command, the bot:

1. Pops the next video from the queue
2. Updates the current playing video
3. Discovers Chromecast devices on your network using `dns-sd`
4. Connects to a specified or default Chromecast device
5. Casts the YouTube video to the device
6. Tracks the video in history

If no Chromecast device is specified, the bot will use the first available device it finds on your network.

## Future Enhancements

- [x] a message containing a youtube link should automatically be added to the queue
- [x] Display video titles and usernames in queue
- [x] Casting functionality to a Chromecast/TV
- [x] `/current` to display the video playing now
- [x] `/history` to see all videos previously played
- [x] Implement actual connection to Chromecast devices
- [ ] Persistent storage for sessions and queue items
- [ ] Prioritize queue so users who haven't gone in a while get queued up sooner
- [ ] Admin controls for managing sessions
- [ ] Support for other video platforms
- [ ] Send message to user when their video is next in line
- [ ] Support for other operating systems

See [ORIGINAL_REQUIREMENTS.md](ORIGINAL_REQUIREMENTS.md) for the initial project requirements and design notes.


