# Local Development

## Quality checks

Run before committing Rust changes:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all -- --test-threads=1 --nocapture
```

## Run locally through Cargo

```bash
cargo run -- inspect
cargo run -- config --endpoint https://example.com/api/rtk/events --token test
cargo run -- once
```

## Build release binary

```bash
cargo build --release
./target/release/rtk-sync inspect
```

## Design constraints

- Use blocking I/O only.
- Do not add async runtimes such as Tokio or async-std.
- Keep sync state outside RTK's SQLite database.
- Open the RTK SQLite database read-only.
- Update checkpoint state only after successful upload.
