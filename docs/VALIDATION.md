# Validation, 2026-10-09

- `cargo check --offline`: passed, using the shared cached target directory.
- `cargo clippy --offline -- -D warnings`: passed.
- Isolated service-module tests against cached dependencies: 3 metadata/media/HTML-normalization tests passed.
- Live service smoke: Fetched today's APOD via NASA's public fallback and decoded the image preview.
- Full native `cargo test`/build and GUI walkthrough: not performed in this pass.

The checked-in `--smoke` entry point provides a repeatable service check once the
binary is built. Parser tests also run under the normal `cargo test` command.

Desktop layout, keyboard navigation, scrolling, and dark/light appearance still
need a native walkthrough. These are development scaffolds, not release-ready apps.
