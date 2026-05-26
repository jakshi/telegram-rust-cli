# tgrc

Telegram Rust CLI — read Telegram messages from the terminal. Supports public channels, private groups, and Saved Messages. Built with [tdlib](https://github.com/tdlib/td) via [tdlib-rs](https://github.com/FedericoBruzzone/tdlib-rs).

## Why

I occasionally stash useful links in Telegram Saved Messages and wanted AI agents to read and sort them. An MCP server is overkill for a local workflow — a CLI plus an agent skill fits better. No existing Telegram CLI suited agent use, so I built one. Turned into a rabbit hole. Rust because it's the reliable, elegant choice for CLIs.

## Install

### Homebrew

```bash
brew install jakshi/tap/tgrc
```

### Prebuilt binary

Download a macOS release binary. Release binaries are statically linked and
bundle all runtime dependencies.

### Build from source

```bash
cargo build --release
```

The first source build downloads a TDLib archive via `tdlib-rs` and needs
network access.

## Authentication

`tgrc` uses Telegram as a user client through TDLib. Unlike the official Telegram
apps, third-party clients need Telegram API app credentials: `api_id` and
`api_hash`. Create your own credentials once, keep `api_hash` private, and store
them in your local config.

1. Get API credentials from [my.telegram.org/apps](https://my.telegram.org/apps):
   - Log in with your phone number
   - Go to "API development tools"
   - Create an application to get `api_id` and `api_hash`

2. Create config file at `~/.config/tgrc/config.toml`:

```toml
[telegram]
api_id = 12345678
api_hash = "your_api_hash_here"
```

3. Log in:

```bash
tgrc auth login
```

4. Complete Telegram login:
   - Enter your phone number
   - Enter the verification code sent by Telegram
   - Enter your 2FA password if your account has one

Authenticate once. TDLib stores the session locally, and future commands reuse
it. Run `tgrc auth logout` to revoke the Telegram session and delete local
session data.

## Commands

### `tgrc auth` — Manage authentication

```bash
tgrc auth login       # log in to Telegram (interactive)
tgrc auth logout      # log out and delete session data
tgrc auth status      # check if you're logged in
```

### `tgrc list` — List your chats

```bash
tgrc list                # list your 50 most recent chats with IDs
tgrc list --limit 100    # list more chats
```

Output:

```
ID               TITLE
------------------------------------------------------------
777000           Telegram
-1001234567890   Example Group
123456789        Alex Example
```

Use the ID from this list to read private chats/groups.

### `tgrc read` — Read messages from a chat

```bash
tgrc read saved                    # read your Saved Messages
tgrc read durov                    # read a public channel by username
tgrc read -- -1001234567890        # read a private group by chat ID
tgrc read                          # read the default chat from config
```

The `chat` argument accepts:
- `saved` — your Saved Messages
- A public channel username (without `@`)
- A numeric chat ID (use `tgrc list` to find it; use `--` before negative IDs)

#### Pagination

```bash
tgrc read durov --limit 5              # latest 5 messages
tgrc read durov --limit 5 --skip 5     # next 5 messages (6-10)
tgrc read durov --limit 5 --skip 10    # messages 11-15
tgrc read durov --limit 5 --skip 3     # 5 messages, skipping the 3 most recent
```

### `tgrc status` — Diagnostics

```bash
tgrc status
```

Output:

```
tgrc v0.1.1
Config:              /Users/you/.config/tgrc/config.toml
Session data:        /Users/you/Library/Application Support/tgrc/tdlib
TDLib log verbosity: 0
Auth:                logged in
```

## Config

`tgrc` loads config from `~/.config/tgrc/config.toml`, falling back to `~/Library/Application Support/tgrc/config.toml`, then `./config.toml`.

The `[channel]` section is optional; pass arguments via CLI instead.

```toml
[telegram]
api_id = 12345678
api_hash = "your_api_hash_here"
# tdlib_log_verbosity = 0  # 0 = silent (default), 1-5 = increasing detail

[channel]
chat = "durov"        # default chat: username, chat ID, or "saved"
message_limit = 20    # default number of messages to fetch
```

| Field | Description |
|-------|-------------|
| `telegram.api_id` | Your Telegram API ID (number or string) |
| `telegram.api_hash` | Your Telegram API hash |
| `telegram.tdlib_log_verbosity` | TDLib log level: 0 = silent (default), 1-5 for debugging |
| `channel.chat` | Default chat: username, chat ID, or `"saved"` |
| `channel.message_limit` | Default number of messages to fetch (default: 20) |
