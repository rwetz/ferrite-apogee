# Apogee

NASA Astronomy Picture of the Day in Ferrite dither.

A working development scaffold, not a published release.

## Run

Requires Rust 1.92 or later and the normal GPUI native toolchain. Ferrite Design
is pinned to a published GitHub commit and fetched automatically by Cargo.

```powershell
git clone https://github.com/rwetz/ferrite-apogee.git
cd ferrite-apogee
cargo run
```

Set `NASA_API_KEY` in your own environment for your NASA quota. Without it,
Apogee uses NASA's limited `DEMO_KEY`. When the classic endpoint is unavailable,
it falls back to NASA's public `science.nasa.gov` APOD feed. Never put your real key in a config file or source.

## Current slice

- Fetch today's APOD title, date, credit, explanation, and media URL.
- Download and dither image entries with bounded download/decode limits.
- Recognize video entries and provide a browser link.
- Keep the NASA key in the environment, with sanitized error messages.

The title-bar menu and Ctrl+Shift+P expose actions and scheme/appearance changes.
`FERRITE_SCHEME`, `FERRITE_APPEARANCE`, and `FERRITE_FPS` supply launch defaults.
These scaffolds do not yet persist their own theme or use the planned shared settings store.

## Next

Date browsing, local cache, ASCII mode, export, and a Darkroom handoff.
Video entries are opened externally. The preview is deliberately bounded and is not a full-resolution export.

## API reference

[Official integration documentation](https://github.com/nasa/apod-api)

## Validation

```powershell
cargo check
cargo clippy -- -D warnings
cargo test
cargo run -- --smoke           # verify the real service without opening a window
```

Compilation and service tests do not establish native window, scrolling, or device quality.
