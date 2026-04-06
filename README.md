# StormStar

Lightweight RPM and APT content management for edge deployments. Single static Rust binary providing repository management for both YUM (RPM) and APT (Deb) ecosystems, content views, lifecycle environments, host registration, and errata tracking — no external database or runtime dependencies.

## Features

- **RPM repository sync** from upstream mirrors (repomd.xml, primary.xml.gz, updateinfo.xml)
- **APT (Deb) repository sync** from upstream mirrors (Release, Packages.gz per component/arch)
- Standalone errata sync — re-fetch errata across all synced RPM repos independently
- Content views with include/exclude filters (name, arch, version glob matching) — works for both RPM and deb
- Lifecycle environments (Library → Dev → Test → Prod) with promotion chain
- Host registration with activation keys (usage limits, auto-key generation)
- Errata tracking (security, bugfix, enhancement) with CVE mapping (RPM repos)
- **Yum-compatible HTTP repo serving** (Pulp-compatible URL layout at `/pulp/repos/`)
- **APT-compatible HTTP repo serving** (dists/pool layout at `/pulp/deb/`)
- Web UI with HTMX interactivity (Dracula dark theme, content type selection, type badges)
- CLI management (repo, cv, env, host, key, errata commands) with deb-specific flags
- Embedded database (native_db/redb — zero external dependencies)
- TLS support via rustls (no OpenSSL dependency)
- Static musl binary — single file deployment, ~15 MB stripped

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

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    StormStar                         │
├───────────┬──────────┬──────────┬───────────────────┤
│  Web UI   │ REST API │   CLI    │  Content Serving  │
│  (HTMX)   │ (axum)   │ (clap)  │  (yum+apt compat) │
├───────────┴──────────┴──────────┴───────────────────┤
│              Content Engine                          │
│  repo sync · errata sync · view compose · lifecycle │
├─────────────────────────────────────────────────────┤
│              Host Management                         │
│  registration · inventory · activation keys          │
├─────────────────────────────────────────────────────┤
│              Embedded Database (native_db/redb)      │
│  13 models · zero external dependencies              │
└─────────────────────────────────────────────────────┘
```

## Web UI

The web UI runs at the root URL (`/`) with 7 pages:

| Page | URL | Features |
|------|-----|----------|
| Dashboard | `/` | Stat cards (clickable), system overview |
| Repositories | `/ui/repos` | Create, delete, sync, status badges |
| Content Views | `/ui/views` | Create, delete, publish versions |
| Environments | `/ui/envs` | Create, delete, position chain |
| Hosts | `/ui/hosts` | Delete, errata badges, package counts |
| Errata | `/ui/errata` | Sync all, type/severity breakdown |
| Activation Keys | `/ui/keys` | Create, delete, usage tracking |

All actions use HTMX for inline interactivity — no full page reloads.

## CLI

```bash
# Repository management (YUM/RPM)
stormstar repo list
stormstar repo create --product-id <id> --name "CentOS Base" --url "https://mirror.centos.org/..."
stormstar repo sync <repo-id>
stormstar repo delete <repo-id>

# Repository management (APT/Deb)
stormstar repo create --product-id <id> --name "Ubuntu Jammy" \
  --url "http://archive.ubuntu.com/ubuntu" \
  --content-type deb --codename jammy --components "main,universe" --architectures "amd64,arm64"

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
stormstar host errata <host-id>

# Errata
stormstar errata list
stormstar errata sync

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
| Errata | `GET /errata?repo_id=&erratum_type=&severity=`, `GET /errata/:id`, `POST /errata/sync` |
| Activation Keys | `GET/POST /activation_keys`, `GET/PUT/DELETE /activation_keys/:id` |
| Sync Plans | `GET/POST /sync_plans`, `GET/PUT/DELETE /sync_plans/:id` |

## Repository Serving

### YUM (RPM) Repos

Synced RPM repos are served at Pulp-compatible URLs:

```
/pulp/repos/<org>/<env>/<cv>/custom/<product>/<repo>/repodata/repomd.xml
/pulp/repos/<org>/<env>/<cv>/custom/<product>/<repo>/Packages/<letter>/<filename>.rpm
```

### APT (Deb) Repos

Synced deb repos are served with standard APT layout:

```
/pulp/deb/<org>/<env>/<cv>/custom/<product>/<repo>/dists/<codename>/Release
/pulp/deb/<org>/<env>/<cv>/custom/<product>/<repo>/dists/<codename>/InRelease
/pulp/deb/<org>/<env>/<cv>/custom/<product>/<repo>/dists/<codename>/<component>/binary-<arch>/Packages
/pulp/deb/<org>/<env>/<cv>/custom/<product>/<repo>/dists/<codename>/<component>/binary-<arch>/Packages.gz
/pulp/deb/<org>/<env>/<cv>/custom/<product>/<repo>/pool/<component>/<prefix>/<source>/<filename>.deb
```

Configure on Debian/Ubuntu hosts:
```bash
echo "deb http://stormstar:8585/pulp/deb/myorg/production/base_os/custom/ubuntu/jammy_main jammy main" \
  > /etc/apt/sources.list.d/stormstar.list
```

Packages are proxied on-demand from the upstream repository.

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

# Static musl (Linux x86_64)
cargo build --release --target x86_64-unknown-linux-musl

# ARM64 (edge/MikroTik)
cargo build --release --target aarch64-unknown-linux-musl
```

## Container

```bash
# Build static binary first
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/stormstar

# Build container (scratch base, ~15 MB)
podman build -t stormstar:latest .

# Run
podman run -d -p 8585:8585 -v stormstar-data:/data/stormstar stormstar:latest
```

## CI/CD

GitHub Actions workflow on self-hosted runner (mkube):

- **build-and-test**: cargo build, cargo test, clippy, musl static release
- **container**: podman build from scratch, push to GHCR

Triggers on push to `main` and pull requests.

## Tests

```bash
cargo test
```

24 tests covering: repodata parsing, errata parsing, deb metadata parsing/generation, database CRUD, package NEVRA, roundtrip validation.

## License

MIT
