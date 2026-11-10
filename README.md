# TodoQuoi

Gestionnaire de tâches de bureau (Windows, macOS), local-first, synchronisé par un dossier de
drive cloud. Rust (Tauri 2, architecture hexagonale) + Vue 3 / TypeScript.

- Spécification : [`docs/01-specification-fonctionnelle.md`](docs/01-specification-fonctionnelle.md)
- Architecture : [`docs/02-architecture.md`](docs/02-architecture.md) · décisions : [`docs/adr/`](docs/adr/README.md)
- Rust pour développeur TypeScript : [`docs/03-guide-rust-pour-dev-ts.md`](docs/03-guide-rust-pour-dev-ts.md)
- **Guide du développeur** (installation, lancement, tests, recettes, débogage) : [`docs/04-guide-developpeur.md`](docs/04-guide-developpeur.md)
- Conventions : [`CLAUDE.md`](CLAUDE.md) · POC : [`poc/README.md`](poc/README.md)

## Prérequis
Rust ≥ 1.90 (`rustup`), Node.js ≥ 22, pnpm. macOS : Xcode Command Line Tools ; Windows : Build Tools MSVC.
Installation pas à pas : [guide du développeur §1](docs/04-guide-developpeur.md#1-installer-lenvironnement).

## Commandes

Installer les dépendances du front :
```bash
pnpm install
```

Lancer l'application (Vite + Tauri) :
```bash
pnpm dev
```

Dossier de données : dans l'app, `⌘,` / `Ctrl+,` (drives détectés ou chemin collé). Pour un essai
ponctuel sans toucher à ce réglage, la variable `TODOQUOI_DATA_DIR` est prioritaire — macOS / Linux :
```bash
TODOQUOI_DATA_DIR="$HOME/Library/Mobile Documents/com~apple~CloudDocs/TodoQuoi" pnpm dev
```
Windows (PowerShell) :
```bash
$env:TODOQUOI_DATA_DIR="$env:OneDrive\TodoQuoi"; pnpm dev
```
Sans variable ni réglage : dossier local à l'appareil (`…/fr.todoquoi.desktop/espace-local`).

Interface seule dans un navigateur, données fictives en mémoire (sans compiler le Rust) :
```bash
pnpm ui:dev
```

Vérifications avant de committer :
```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```
```bash
pnpm lint && pnpm test
```
`pnpm lint` = typage (`vue-tsc`) + ESLint + mise en forme (Prettier). Pour corriger automatiquement ce qui peut l'être :
```bash
pnpm --filter ui lint:fix
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
crates/adapters/*      system (horloge, UUID v7), fs_markdown (stockage), note_markdown (notes), memory_index (recherche)
src-tauri/             commandes Tauri + composition root
ui/                    interface Vue (domain, ports, infrastructure, application, presentation)
```
