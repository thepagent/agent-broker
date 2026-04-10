# Format code
fmt:
    cargo fmt

# Run all lints (format check + clippy)
lint:
    cargo fmt --check
    cargo clippy -- -D warnings
