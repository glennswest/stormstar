# Changelog

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
