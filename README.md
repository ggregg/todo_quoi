# TodoQuoi

Gestionnaire de tâches de bureau (Windows, macOS), local-first, synchronisé par un dossier de
drive cloud. Rust (Tauri 2, architecture hexagonale) + Vue 3 / TypeScript.

- Spécification : [`docs/01-specification-fonctionnelle.md`](docs/01-specification-fonctionnelle.md)
- Architecture : [`docs/02-architecture.md`](docs/02-architecture.md) · décisions : [`docs/adr/`](docs/adr/README.md)
- Rust pour développeur TypeScript : [`docs/03-guide-rust-pour-dev-ts.md`](docs/03-guide-rust-pour-dev-ts.md)
- Conventions : [`CLAUDE.md`](CLAUDE.md) · POC : [`poc/README.md`](poc/README.md)

## Prérequis
Rust stable (`rustup`), Node.js, pnpm. macOS : Xcode ; Windows : Build Tools MSVC.

## Commandes

Installer les dépendances du front :
```bash
pnpm install
```

Lancer l'application (Vite + Tauri) :
```bash
pnpm dev
```

Choisir le dossier de données (en attendant l'écran de choix du drive, I5) — macOS / Linux :
```bash
TODOQUOI_DATA_DIR="$HOME/Library/Mobile Documents/com~apple~CloudDocs/TodoQuoi" pnpm dev
```
Windows (PowerShell) :
```bash
$env:TODOQUOI_DATA_DIR="$env:OneDrive\TodoQuoi"; pnpm dev
```
Sans cette variable : dossier local à l'appareil (`…/fr.todoquoi.app/espace-local`).

Interface seule dans un navigateur, données fictives en mémoire (sans compiler le Rust) :
```bash
pnpm ui:dev
```

Vérifications avant de committer :
```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```
```bash
pnpm ui:check && pnpm ui:test
```

Documentation du code Rust, navigable :
```bash
cargo doc --workspace --no-deps --open
```

## Organisation
```
crates/domain          règles métier pures
crates/application     cas d'usage + ports
crates/testing         faux ports, tests de contrat
crates/adapters/*      system (horloge, UUID v7), fs_markdown (stockage)
src-tauri/             commandes Tauri + composition root
ui/                    interface Vue (domain, ports, infrastructure, application, presentation)
```
