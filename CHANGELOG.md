# Changelog

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

### 2026-04-05
- **feat:** Errata sync engine — standalone errata fetch across all synced repos (API + CLI + UI)
- **feat:** HTMX interactive UI — create/delete forms on all pages (repos, views, envs, hosts, keys)
- **feat:** Errata page stats breakdown (security/bugfix/enhancement counts)
- **feat:** Dashboard clickable stat cards, activation key count added
- **feat:** CLI `stormstar errata list|sync` commands
- **chore:** CI/CD workflow for mkube self-hosted runner (GitHub Actions, podman, GHCR)
- **chore:** Containerfile (stormdbase, stormd supervised, aarch64-musl)
- **chore:** deploy/stormd.toml — stormd supervisor config with liveness probe and UI proxy
- **chore:** deploy/stormstar.yaml — mkube Pod manifest with vkube annotations
- **chore:** .cargo/config.toml — aarch64-linux-musl cross-linker
- **fix:** Version mismatch in main.rs (0.1.0 → 0.2.0)
- **docs:** Updated README with architecture, CI/CD, container build sections

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
