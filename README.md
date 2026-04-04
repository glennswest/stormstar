# StormStar

Lightweight RPM content management for edge deployments. Single Rust binary (~15MB) providing RPM repository management, content views, lifecycle environments, and host registration.

## Features

- RPM repository sync from upstream mirrors
- Content-addressed package storage (SHA256 deduplication)
- Content views with include/exclude filters
- Lifecycle environments (Library -> Dev -> Test -> Prod)
- Atomic promotion via symlink swap
- Host registration with activation keys
- Errata tracking (security, bugfix, enhancement)
- Yum-compatible HTTP repo serving
- Web UI (Dracula dark theme)
- CLI management
- Embedded database (native_db/redb)
- TLS via rustls (no OpenSSL)

## Quick Start

```bash
# Build
cargo build --release

# Run with default config
./target/release/stormstar serve

# Run with custom config
./target/release/stormstar -c /etc/stormstar/stormstar.toml serve
```

## Configuration

See `config/stormstar.example.toml` for all options.

## API

All endpoints under `/api/v1/`:

| Resource | Endpoints |
|----------|-----------|
| Repositories | `GET/POST /repos`, `GET/PUT/DELETE /repos/:id`, `POST /repos/:id/sync` |
| Content Views | `GET/POST /content_views`, `POST /content_views/:id/publish`, `POST /content_views/:id/promote` |
| Environments | `GET/POST /environments`, `GET/PUT/DELETE /environments/:id` |
| Hosts | `GET/POST /hosts`, `POST /hosts/register` |
| Errata | `GET /errata` |
| Activation Keys | `GET/POST /activation_keys` |
| Sync Plans | `GET/POST /sync_plans` |

## Build for ARM64 (MikroTik)

```bash
cargo build --release --target aarch64-unknown-linux-musl
```

## License

MIT
