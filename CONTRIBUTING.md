# Contributing

Build with Rust 1.96+ and the desktop development libraries for your OS.
Run cargo fmt --check, cargo clippy --locked --all-targets --all-features -- -D warnings,
and cargo test --locked --all-features. For independent array validation, install
NumPy, build with --features bench and run python native/validate_numpy.py.

Interface translations are in locales/. Keep the same keys and placeholders in
all six catalogues. RTL display must not mutate original array values or copies.
Keep data storage and analysis independent of the desktop presentation.

Do not commit private datasets, metadata, credentials, toolchains, SDKs or build
outputs. Use synthetic fixtures in tests. Preserve GPL and third-party notices.
See agent.md for decisions and docs/release-status.md for current checks.
