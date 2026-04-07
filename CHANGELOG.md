# Changelog

## [v0.5.0] — 2026-04-07

### Added
- **Full local package downloads** — sync downloads actual RPM/deb files to disk, no more upstream proxy dependency
- **Download engine** — new `src/content/download.rs` with concurrent downloads, SHA256 verification, atomic writes, progress tracking
- **Local disk serving** — packages served from `{data_dir}/repos/{repo_id}/` with streaming via `tokio-util::ReaderStream`; upstream proxy fallback for non-downloaded packages
- **Sync progress tracking** — live `SyncProgress` struct with phase, downloaded/skipped/failed counts, bytes, current package name
- **Progress API** — `GET /api/v1/repos/{id}/sync-progress` returns live download progress during sync
- **Size estimate API** — `GET /api/v1/repos/{id}/size-estimate` returns total/downloaded size in human-readable format
- **Package browser** — new `/ui/repos/{id}/packages` page with search by name, architecture filter dropdown, pagination (50/page), Local/Upstream source badges
- **Component/architecture selectors** — deb catalog shows checkboxes for available components (main, contrib, non-free, etc.) and architectures (amd64, arm64, i386) with sensible defaults
- **Download concurrency config** — `download_concurrency` setting in TOML config (default: 4 parallel downloads)
- **Size and download columns** — repo table shows total size (human-readable) and downloaded/total package counts
- **Sync progress display** — when syncing, HTMX polls progress endpoint every 2s to show live status
- **5 new tests** — `rpm_local_path`, `deb_local_path`, `is_already_downloaded`, `format_bytes` (download engine unit tests)

### Changed
- **Deb catalog defaults** — Debian and Ubuntu repos now default to `main` component and `amd64` architecture only (previously synced all components, causing ~1.28M package count)
- **Package model** — added `downloaded`, `local_path`, `download_size` fields (serde default for backward compat)
- **Repository model** — added `total_size_bytes`, `downloaded_size_bytes`, `downloaded_package_count` fields
- **SyncLog model** — added `packages_downloaded`, `packages_skipped`, `bytes_downloaded`, `total_size_bytes` fields
- **Sync engine** — `sync_repo()` now takes `config` and `progress` params; downloads packages after metadata sync
- **Repo table** — name is now a link to package browser; added Size and Downloaded columns
- **AppState/WebState/ContentState** — all include `ProgressMap` for shared progress tracking
- **Dependencies** — added `tokio-util` 0.7 with `io` feature for streaming file serving

## [v0.4.0] — 2026-04-07

### Added
- **Known repo catalog** — distro-based selector for CentOS 7, Rocky 8/9, AlmaLinux 8/9, RHEL 7/8/9, EPEL 7/8/9, Debian Bookworm, Ubuntu Noble
- **Batch repo creation** — select distro, check repos, create all at once via `POST /ui/repos/create-batch`
- **RHEL CDN auth** — SSL client certificate support (`ssl_client_cert`, `ssl_client_key` fields) for Red Hat CDN repos
- **HTTP Basic Auth** — `username` and `password` fields on Repository for authenticated repo sync
- **Enable/disable repos** — `enabled` boolean field with toggle button in UI; disabled repos skip sync
- **Sync Logs** — new `SyncLog` model (native_model id=14) tracking every sync start/success/failure with timing and counts
- **Sync Logs page** — `/ui/logs` showing sync history with status badges, duration, package/errata counts
- **Relative timestamps** — "just now", "5 minutes ago", "yesterday", "3 days ago" instead of raw RFC3339
- **Dashboard sync log card** — clickable stat card for sync logs count

### Changed
- **UI theme** — replaced Dracula purple/pink with clean dark gray (GitHub dark mode) + blue accents
- **Repos page** — redesigned with catalog selector, custom form, auth fields, enable/disable toggle
- **Sync engine** — uses `build_client()` + `apply_auth()` helpers for SSL cert and Basic Auth
- **API** — `CreateRepo` and `UpdateRepo` accept `enabled`, `username`, `password`, `ssl_client_cert`, `ssl_client_key`
- **CLI** — `repo create` gains `--username`, `--password`, `--disabled` flags

## [v0.3.0] — 2026-04-05

### Added
- **APT (Debian/Ubuntu) repository support** — full deb content lifecycle alongside RPM
- **Deb metadata parsing** — RFC822 Packages file parser, Release file parser, version parsing (epoch:upstream-revision)
- **Deb metadata generation** — generate Packages and Release files from DB records
- **Deb sync engine** — fetch dists/Release + Packages.gz for each component/arch combo, store in DB
- **APT serving routes** — 5 new endpoints at `/pulp/deb/` for Release, InRelease, Packages, Packages.gz, pool proxy
- **Content type selection** — repos can be "yum" (RPM) or "deb" (APT) with type-specific fields
- **Deb-specific model fields** — codename, components, architectures on Repository
- **CLI deb support** — `--content-type deb --codename --components --architectures` flags on repo create
- **Web UI deb support** — content type dropdown, conditional deb fields, type badge column (purple=yum, cyan=deb)
- **7 new tests** — parse_packages, parse_deb_version, parse_release, generate_packages, generate_release, roundtrip_packages, decompress_packages_gz
- **Errata guard** — errata sync skips deb repos gracefully (no updateinfo.xml for APT)
- **Content view filter** — FilterContentType::Deb variant, package filters work for both RPM and deb

### Changed
- Repository model extended with optional deb fields (backward compatible via `#[serde(default)]`)
- Sync engine dispatches to yum or deb based on content_type
- Errata sync filters out deb repos

## [Unreleased]


## [v0.2.0] — 2026-04-05

### Added
- **CRUD API** — 9 resources fully wired to native_db: organizations, products, repositories, content views, lifecycle environments, hosts, activation keys, sync plans, errata
- **Health endpoint** at `/api/v1/health`
- **Repository sync engine** — fetch repomd.xml, parse primary.xml.gz, download package metadata, parse updateinfo.xml for errata/CVEs
- **Repodata generation** — generate repomd.xml and primary.xml.gz from database for yum-compatible serving
- **Yum-compatible repo serving** — Pulp-compatible URL layout at `/pulp/repos/<org>/<env>/<cv>/custom/<product>/<repo>/`, RPM proxy from upstream
- **Content view engine** — compose views from repos, apply include/exclude filters (name/arch/version glob matching), publish versioned snapshots
- **Lifecycle environment promotion** — promote CV versions through env chain (Library → Dev → Test → Prod), default chain creation helper
- **Host registration** — register hosts via activation keys with usage limits, package inventory tracking, errata applicability computation
- **Activation key management** — create keys with usage limits, automatic key string generation
- **CLI commands** — all 5 command groups (repo, cv, env, host, key) wired to API via reqwest
- **HTMX web UI** — Dracula dark theme, 7 pages: dashboard stats, repositories (sync button), content views (publish button), lifecycle environments, hosts (errata badges), errata browser (type/severity badges), activation keys
- **17 tests** — repodata parsing, errata parsing, database CRUD, package NEVRA, generation roundtrip

## [v0.1.0] — 2026-04-04

### Added
- **Phase 0 scaffolding** — project skeleton with Cargo.toml, config parsing, 13 database models
- **Database models** — Organization, Product, Repository, Package, Erratum, ContentView, ContentViewVersion, ContentViewFilter, LifecycleEnvironment, Host, ActivationKey, SyncPlan, HostCollection
- **API route stubs** — axum REST router with 7 resource groups
- **CLI structure** — clap-based CLI with 5 command groups
- **Module tree** — content (repo, repodata, errata, view, lifecycle, serve), host (inventory, keys), web (7 pages + style)
