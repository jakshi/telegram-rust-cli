# Changelog

## v0.2.0

First public release.

- publish the project from a clean public repository history
- add public CI checks for formatting, tests, clippy, and RustSec advisories
- replace private Homebrew release asset handling with public release URLs
- document Telegram API credential setup and authentication flow
- clarify macOS prebuilt binary support and source build requirements
- add public package metadata
- remove private development notes and local tool configuration from project tracking

## v0.1.2

Fix: Homebrew-installed binary crashed with `dyld: Library not loaded: libtdjson`.

- upgrade tdlib-rs from 1.3 to 1.4 and enable the `static` feature — TDLib is now
  statically linked into the binary, eliminating the runtime dependency on `libtdjson.dylib`

## v0.1.1

Patch release.

- add the MIT `LICENSE` file and package metadata
- make the release workflow tolerate reruns when the GitHub release already exists

## v0.1.0

Initial release.

- **auth** — login, logout (revokes server session), status
- **list** — list chats with IDs (public channels, private groups, DMs)
- **read** — read messages from any chat by username, chat ID, or `saved`; supports `--limit` and `--skip` for pagination
- **status** — diagnostics (config path, session path, TDLib verbosity, auth state)
- TOML config with flexible `api_id` (string or number), optional `[channel]` defaults
- TDLib log suppression (silent by default, configurable via `tdlib_log_verbosity`)
- Local timezone timestamps
- GitHub Actions release workflow with Homebrew tap auto-update
