# StormStar

Lightweight RPM content management for edge deployments. Single Rust binary providing RPM repository management, content views, lifecycle environments, and host registration.

## Features

- RPM repository sync from upstream mirrors (repomd.xml, primary.xml.gz, updateinfo.xml)
- Content views with include/exclude filters (name, arch, version glob matching)
- Lifecycle environments (Library → Dev → Test → Prod) with promotion chain
- Host registration with activation keys (usage limits, auto-key generation)
- Errata tracking (security, bugfix, enhancement) with CVE mapping
- Yum-compatible HTTP repo serving (Pulp-compatible URL layout)
- Web UI (HTMX, Dracula dark theme, 7 pages)
- CLI management (repo, cv, env, host, key commands)
- Embedded database (native_db/redb — no external database needed)
- TLS support via rustls (no OpenSSL dependency)

## Quick Start

```bash
# Build
cargo build --release

# Run server (default: 0.0.0.0:8585)
./target/release/stormstar serve

# Run with custom config
./target/release/stormstar -c /path/to/stormstar.toml serve
```

Access the web UI at `http://localhost:8585/`

## CLI

```bash
# Repository management
stormstar repo list
stormstar repo create --product-id <id> --name "CentOS Base" --url "https://mirror.centos.org/..."
stormstar repo sync <repo-id>

# Content views
stormstar cv list
stormstar cv create --org-id <id> --name "Base OS"
stormstar cv publish <cv-id>
stormstar cv promote <cv-id> --version 1 --env <env-id>

# Lifecycle environments
stormstar env list
stormstar env create --org-id <id> --name "Development" --prior <library-id>

# Host management
stormstar host list
stormstar host register --key <activation-key> --hostname server1.example.com

# Activation keys
stormstar key list
stormstar key create --org-id <id> --name "dev-key" --env <env-id> --cv <cv-id>
```

## API

All endpoints under `/api/v1/`:

| Resource | Endpoints |
|----------|-----------|
| Health | `GET /health` |
| Organizations | `GET/POST /organizations`, `GET/PUT/DELETE /organizations/:id` |
| Products | `GET/POST /products`, `GET/PUT/DELETE /products/:id` |
| Repositories | `GET/POST /repos`, `GET/PUT/DELETE /repos/:id`, `POST /repos/:id/sync`, `GET /repos/:id/packages` |
| Content Views | `GET/POST /content_views`, `GET/PUT/DELETE /content_views/:id`, `POST /content_views/:id/publish`, `POST /content_views/:id/promote` |
| Environments | `GET/POST /environments`, `GET/PUT/DELETE /environments/:id` |
| Hosts | `GET/POST /hosts`, `GET/PUT/DELETE /hosts/:id`, `POST /hosts/register` |
| Errata | `GET /errata?repo_id=&erratum_type=&severity=`, `GET /errata/:id` |
| Activation Keys | `GET/POST /activation_keys`, `GET/PUT/DELETE /activation_keys/:id` |
| Sync Plans | `GET/POST /sync_plans`, `GET/PUT/DELETE /sync_plans/:id` |

## Yum Repository Serving

Synced repos are served at Pulp-compatible URLs:

```
/pulp/repos/<org>/<env>/<cv>/custom/<product>/<repo>/repodata/repomd.xml
/pulp/repos/<org>/<env>/<cv>/custom/<product>/<repo>/Packages/<letter>/<filename>.rpm
```

RPMs are proxied on-demand from the upstream repository.

## Configuration

```toml
listen = "0.0.0.0:8585"
data_dir = "/data/stormstar"
organization = "MyOrg"
log_level = "info"

[tls]
cert = "/path/to/cert.pem"
key = "/path/to/key.pem"
```

## Build

```bash
# Development
cargo build

# Release (native)
cargo build --release

# Static musl (Linux)
cargo build --release --target x86_64-unknown-linux-musl

# ARM64 (edge/MikroTik)
cargo build --release --target aarch64-unknown-linux-musl
```

## Tests

```bash
cargo test
```

17 tests covering: repodata parsing, errata parsing, database CRUD, package NEVRA, repodata generation roundtrip.

## License

MIT
