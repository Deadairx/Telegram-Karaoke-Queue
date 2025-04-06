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
- `/start`: Create a new karaoke session
- `/join [code]`: Join an existing session with a code
- `/add [youtube_url]`: Add a YouTube link to the queue
- `/queue`: View current queue
- `/leave`: Leave current session

## Future Enhancements

- [ ] Persistent storage for sessions and queue items
- [ ] Casting functionality to a Chromecast/TV
- [ ] Prioritize queue so users who haven't gone in a while get queued up sooner
- [ ] Admin controls for managing sessions
- [ ] Display video thumbnails and titles in queue
- [ ] Support for other video platforms

## Original Requirements

The idea behind this is I would start a session and users can join the session
by giving the Telegram Bot a session code, after that, they can share
youtube links and it would add it to a queue

The queue service needs to be able to play videos to a casted device

Probably gonna need to validate that the link is a valid youtube link first

#enhancement validate the link is not duplicate
#enhancement prioritize queue so that users who haven't gone in a while get queued up sooner

Needs crashing contingency plan
    Store user ids so that on server restart users don't need to enter the session code again
    Keep a history of links "played" incase one gets skipped for some reason


