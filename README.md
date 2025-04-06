# Karaoke Queue Bot

This Telegram bot allows users to join a karaoke session and add YouTube videos to a shared queue for playback.

## Features

- Create and join karaoke sessions with unique session codes
- Add YouTube videos to a shared queue
- View the current queue
- Automatic validation of YouTube links
- Duplicate link detection
- Session persistence across bot restarts (coming soon)
- Cast functionality to play videos (coming soon)
- Queue prioritization for users who haven't gone in a while (coming soon)

## Getting Started

### Prerequisites

- Rust and Cargo installed
- Telegram Bot token (obtainable from @BotFather)

### Setup

1. Clone the repository
2. Create a `.env` file in the root directory with:
```
TELEGRAM_BOT_TOKEN=your_telegram_bot_token_here
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

## Bot Commands

- `/help`: Display help information
- `/start`: Display help information
- `/start-session`: Create a new karaoke session
- `/join [code]`: Join an existing session with a code
- `/add [youtube_url]`: Add a YouTube link to the queue
- `/queue`: View current queue
- `/leave`: Leave current session

## Future Enhancements

- [x] a message containing a youtube link should automatically be added to the queue
- [ ] `/current` to display the video playing now
- [ ] `/backlog` or `/history` to see all videos previously played
- [ ] Persistent storage for sessions and queue items
- [ ] Casting functionality to a Chromecast/TV
- [ ] Prioritize queue so users who haven't gone in a while get queued up sooner
- [ ] Admin controls for managing sessions
- [ ] `/pop` to diplay next link and queue and remove it from the queue
- [ ] Display video thumbnails and titles in queue
- [ ] Support for other video platforms

See [ORIGINAL_REQUIREMENTS.md](ORIGINAL_REQUIREMENTS.md) for the initial project requirements and design notes.


