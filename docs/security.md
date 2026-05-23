# Security Notes

- HTTPS is required by default.
- Plain HTTP requires explicit `--allow-insecure-http`.
- Tokens are never printed by the sync client.
- If you do not want tokens written to `config.toml`, use `--token-env` and provide the token via environment variable.
- `rtk-sync` opens the RTK SQLite database read-only.
- `rtk-sync` never writes sync metadata into RTK's SQLite database.
- Uploaded events contain command metadata and token counts. Do not enable raw command output upload unless that feature is explicitly designed and reviewed later.
