# TodoQuoi — conventions du projet

Gestionnaire de tâches client lourd, local-first, synchronisé via un dossier de drive cloud.
Rust (Tauri 2) + Vue 3 / TypeScript. Cibles : Windows + macOS, puis Linux, peut-être iOS.

Références (à lire avant de coder) :
- `docs/01-specification-fonctionnelle.md` — quoi (règles métier R1–R12)
- `docs/02-architecture.md` — comment (hexagonal, couches, ports, conventions §9)
- `docs/03-guide-rust-pour-dev-ts.md` — sous-ensemble de Rust autorisé
- `maquette/index.html` — maquette interactive de l'UI

## Mainteneur
Senior Vue/TypeScript, **débutant en Rust**. Le code Rust doit pouvoir être repris par un humain dans ce profil.

## Règles impératives

### Architecture hexagonale
- Dépendances **vers le centre uniquement** : `domain` ← `application` ← `adapters` ← `src-tauri` (composition root).
- `domain` : pur, sans E/S ; dépendances limitées à `thiserror` et `serde`.
- `application` : un cas d'usage par fichier ; dépend de **ports** (traits) uniquement.
- Un adaptateur implémente un port et traduit ses erreurs techniques en erreurs du port.
- Aucune règle métier dans `src-tauri` ni dans le front : le Rust **décide**, l'UI **affiche**.
- Front : les composants passent par des ports (`ui/src/ports`), jamais `invoke()` directement.

### SOLID / Clean Code
- Une responsabilité par type / fichier ; ports petits ; injection par `Arc<dyn Port>`.
- Fonctions courtes (< 25 lignes visé), retours anticipés, noms révélant l'intention, pas d'abréviation.
- Pas de paramètre booléen (`close()` / `reopen()`, pas `set_done(bool)`).
- Newtypes et enums pour rendre les états invalides irreprésentables.
- Pas de port « au cas où » ; pas de sur-ingénierie.

### Code pédagogique (Rust)
- `//!` en tête de chaque fichier : rôle + place dans l'hexagone.
- `///` sur tout élément public (quoi, pourquoi, erreurs, exemple si utile).
- `// [rust]` à la première occurrence d'un concept Rust dans un fichier, avec l'équivalent TS.
- Sous-ensemble simple : structs propriétaires (`String`, `Vec`), pas de durées de vie explicites sauf nécessité documentée, pas de macros maison, domaine/application **synchrones**.
- Erreurs avec `thiserror` ; côté `src-tauri`, un `UiError` sérialisable (pas d'`anyhow`).
- Pas d'`unwrap()`/`expect()` hors tests ; pas d'`unsafe` hors `adapters/cloud_sync` (avec `// SAFETY:`).
- Identifiants en **anglais**, commentaires et docs en **français**.
- Décision structurante → ADR dans `docs/adr/`.

### Dépendances — sobriété
- Inventaire et politique : `docs/02-architecture.md` §16. **Aucune dépendance (crate ou paquet npm) hors de cet inventaire** sans l'y ajouter d'abord avec sa justification, et sans accord de Greg.
- Avant d'ajouter une dépendance : peut-on l'écrire simplement et sûrement (< ~50 lignes testées) ? Tauri ou la plateforme web le fournit-il déjà ? Si oui, pas de dépendance.
- Jamais de crypto, de parseur CommonMark ou d'API système réécrits à la main.

### Tests
- Domaine : tests unitaires + `proptest`. Application : fakes de `crates/testing`. Adaptateurs : tests de contrat partagés avec les fakes.
- Avant de considérer une tâche finie : `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, et côté UI `pnpm lint`, `pnpm test`.

### Portabilité
- Windows et macOS au quotidien : chemins via `std::path` / `directories`, jamais de chemin en dur ; fichiers en LF ; attention à la casse.

### Sécurité
- Aucune installation (outil, crate système, package global) sans accord explicite.
- Ne jamais lire ni committer de secrets ; aucune donnée réelle dans les fixtures.
