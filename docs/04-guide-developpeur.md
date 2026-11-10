# Guide du développeur — installer, lancer, tester, faire évoluer TodoQuoi

> Public : développeur qui reprend le projet (profil visé : senior TS/Vue, débutant Rust).
> Ce guide est **tenu à jour à chaque incrément** (règle du `CLAUDE.md`).
> Le *quoi* est dans [`01`](01-specification-fonctionnelle.md), le *pourquoi* dans
> [`02`](02-architecture.md) et les [ADR](adr/README.md), le Rust utile dans [`03`](03-guide-rust-pour-dev-ts.md).

## Sommaire
1. [Installer l'environnement](#1-installer-lenvironnement)
2. [Lancer l'application](#2-lancer-lapplication)
3. [Tester et vérifier](#3-tester-et-vérifier)
4. [Se repérer dans le code](#4-se-repérer-dans-le-code)
5. [Parcours d'une action de bout en bout](#5-parcours-dune-action-de-bout-en-bout)
6. [Recettes : faire évoluer le code](#6-recettes--faire-évoluer-le-code)
7. [Déboguer](#7-déboguer)
8. [Définition de « terminé » et flux Git](#8-définition-de--terminé--et-flux-git)
9. [Problèmes connus](#9-problèmes-connus)

---

## 1. Installer l'environnement

### Outils requis

| Outil | Version | Rôle |
|---|---|---|
| Rust (`rustup`, `cargo`) | ≥ 1.90 (édition 2024, cf. `Cargo.toml`) | Cœur, adaptateurs, coquille Tauri |
| Node.js | ≥ 22 LTS | Outillage du front (Vite, vitest) — **pas** utilisé à l'exécution |
| pnpm | 12.x (épinglé par `packageManager` dans `package.json`) | Paquets du front, scripts |
| macOS : Xcode Command Line Tools | — | Compilateur C, SDK, WebKit |
| Windows : Visual Studio Build Tools (charge « Développement Desktop en C++ ») | 2022 ou + | Compilateur MSVC, SDK Windows |
| Windows : WebView2 | — | Moteur web de l'app ; déjà présent sur Windows 11 |

Aucun outil global supplémentaire (pas de `tauri-cli` global : il est en dépendance de dev du dépôt).

### macOS

```bash
xcode-select --install
```
```bash
brew install pnpm node
```
Rust par l'installeur officiel `rustup` (la formule Homebrew `rustup` est *keg-only* et demande de
régler le `PATH` à la main) :
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows (PowerShell)

```powershell
winget install Rustlang.Rustup OpenJS.NodeJS.LTS pnpm.pnpm
```
```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive"
```
Rouvrir le terminal, puis `rustup default stable`.

### Récupérer le projet

```bash
git clone <url-du-depot> TodoQuoi && cd TodoQuoi
```
```bash
pnpm install
```
Le premier `pnpm dev` ou `cargo build` télécharge et compile les crates (plusieurs minutes) ; les suivants sont incrémentaux.

Vérifier que tout fonctionne : `cargo test --workspace && pnpm test` doivent être verts.

### Réglages d'éditeur conseillés
- **RustRover** ou **VS Code + rust-analyzer** pour le Rust ; **WebStorm** ou **VS Code + Vue (Official)** pour le front.
- Formatage à l'enregistrement : `rustfmt` (Rust), Prettier (front, config `ui/.prettierrc.json`).
- Fins de ligne : **LF** partout (imposé par `.gitattributes`). Sous Windows : `git config core.autocrlf false`.

---

## 2. Lancer l'application

| Besoin | Commande | Remarque |
|---|---|---|
| App complète (Rust + front, rechargement à chaud du front) | `pnpm dev` | Le Rust est recompilé et l'app relancée à chaque modification Rust |
| Front seul dans un navigateur, données fictives | `pnpm ui:dev` → <http://localhost:5173> | Aucune compilation Rust ; passerelles « en mémoire » |
  | Build de production | `pnpm build` | Binaire `target/release/todoquoi(.exe)`, paquets dans `target/release/bundle/` (installeurs : I7) |
| Build de production, binaire seul | `pnpm tauri build --no-bundle` | Pas d'empaquetage : rien à télécharger (cf. WiX ci-dessous) |
| Doc du code Rust | `cargo doc --workspace --no-deps --open` | Générée depuis les `//!` et `///` |

### Bases de données
Ordre de priorité : variable `TODOQUOI_DATA_DIR` (une seule base), puis les bases ajoutées dans
l'app (`⌘,` / `Ctrl+,` ou « + » du bandeau, enregistrées dans `settings.json` du dossier de
configuration de l'app), sinon un dossier **local** à l'appareil (`<données locales de l'app>/fr.todoquoi.app/espace-local`). Pour essayer
sans toucher au réglage :

macOS / Linux :
```bash
TODOQUOI_DATA_DIR="$HOME/tmp/todoquoi-essai" pnpm dev
```
Windows (PowerShell) :
```powershell
$env:TODOQUOI_DATA_DIR="$env:USERPROFILE\tmp\todoquoi-essai"; pnpm dev
```
**Développer sur un dossier jetable**, jamais sur le dossier réel du drive. Le format des fichiers est
décrit dans `02-architecture.md` §4 ; on peut les ouvrir dans n'importe quel éditeur.

### Coffre et tâches chiffrées (I6, ADR-0010)
- Un coffre **par base**, créé au premier clic sur 🔒 (panneau de détail) ; cadenas sur la pastille
  de la base pour verrouiller / déverrouiller. En développement, utiliser un mot de passe jetable
  sur un dossier jetable (`TODOQUOI_DATA_DIR`) : **un mot de passe perdu rend les notes
  chiffrées irrécupérables**, y compris pour nous.
- Le coffre est décrit dans `manifest.json` (`vault`) ; une tâche chiffrée garde son fichier
  `tasks/<id>.md` : tout l'en-tête en clair (titre compris), seule la note est chiffrée (format :
  architecture §4.2).
- « Se souvenir sur cet appareil » range la clé dans le Trousseau macOS / Gestionnaire
  d'identification Windows (service `TodoQuoi`, compte `vault-<key_id>`) — **pas en
  développement**, sauf `TODOQUOI_KEYCHAIN=1` (voir Problèmes connus). Pour l'oublier :
  déverrouiller en décochant la case, ou supprimer l'entrée dans Trousseaux d'accès / Gestionnaire
  d'identification.
- **Médias d'une note chiffrée** : chiffrés aussi, nommés `media/<empreinte>.<ext>.tqe` (empreinte
  HMAC avec la clé du coffre). Chiffrer / déchiffrer une tâche renomme ses médias et efface les
  anciennes copies que plus aucune note ne cite. Ouvrir une pièce jointe chiffrée passe par une
  copie déchiffrée dans `$TMPDIR/todoquoi-ouverts/` (macOS) ou `%TEMP%\todoquoi-ouverts\`
  (Windows), effacée au verrouillage d'un coffre et au lancement suivant.
- **Verrouillage automatique** : 15 min sans activité par défaut, réglable dans la fenêtre
  « Bases » (`vaultAutoLockMinutes` dans `settings.json` de l'appareil, `0` = jamais). Minuteur
  Rust (`src-tauri/src/auto_lock.rs`, toutes les 20 s) ; l'interface signale frappes et clics
  (`vault_activity`, au plus toutes les 30 s par fenêtre). Les coffres gardés dans le trousseau
  ne se verrouillent pas seuls. Pour l'essayer vite : choisir 5 min.
- **Changer le mot de passe** : menu de la pastille de la base (« ⋯ » ou clic droit). Seule
  l'enveloppe de la clé change dans `manifest.json`.
- Les fenêtres de tâche détachées ont les mêmes boutons de coffre ; toutes les fenêtres suivent
  l'état des coffres par l'événement `vaults-changed`.
- Navigateur (`pnpm ui:dev`) : coffre **simulé** (`InMemoryVaultGateway`), rien n'est chiffré.

### Ce qui tourne à l'exécution
Seul le binaire Rust : il embarque le front compilé et le sert à sa webview. Node/Vite n'existent
qu'en développement (cf. `02-architecture.md` §1.1).

---

## 3. Tester et vérifier

### Où sont les tests

| Couche | Emplacement | Outil | Ce qu'on y teste |
|---|---|---|---|
| Domaine | `mod tests` en bas de chaque fichier de `crates/domain/src` | `cargo test` (tables de cas ; `proptest` écarté, §16 de l'archi) | Règles pures (R1–R12), invariants des types |
| Application | `crates/application/tests/use_cases.rs` | `cargo test` + fakes de `crates/testing` | Cas d'usage, sans disque ni horloge réelle |
| Contrat des ports | `crates/testing/src/contract.rs` | appelé par le fake **et** par l'adaptateur réel | Même garanties pour le faux et le vrai |
| Adaptateurs | `crates/adapters/*/tests/` | `cargo test` | Fichiers réels (dossier temporaire), corpus Markdown |
| Front | `ui/src/**/*.test.ts` | `vitest` | Composables, règles d'affichage, clavier |

### Commandes

| Commande | Effet |
|---|---|
| `cargo test --workspace` | Tous les tests Rust |
| `cargo test -p tq_domain` | Une crate (`tq_application`, `tq_adapter_fs_markdown`, `tq_adapter_note_markdown`…) |
| `cargo test -p tq_domain closure` | Les tests dont le nom contient `closure` |
| `cargo test -- --nocapture` | Affiche les `println!` des tests |
| `pnpm test` | Tests du front (une passe) |
| `pnpm --filter ui exec vitest` | Tests du front en mode surveillance |

### Vérifications obligatoires avant commit

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```
```bash
pnpm lint && pnpm test
```
`pnpm lint` = `vue-tsc` (types) + ESLint + Prettier. Corriger automatiquement : `cargo fmt --all` et
`pnpm --filter ui lint:fix`.

Lints Rust notables (`Cargo.toml`, `clippy.toml`) : `unwrap_used` interdit, `too_many_lines` à 40,
`missing_docs`, `unsafe_code` interdit. Une exception se justifie par
`#[expect(clippy::…, reason = "…")]`, jamais par un `allow` silencieux.

---

## 4. Se repérer dans le code

```
crates/domain/               Règles métier pures : Task, TaskTree, rules/ (closure, hierarchy, trash), search/
crates/application/          ports/ (traits) + use_cases/ (un fichier par cas d'usage) + session, stamper, indexing
crates/testing/              Fakes des ports, TaskBuilder, contrats partagés
crates/adapters/system/      Horloge, UUID v7, nom de l'appareil
crates/adapters/fs_markdown/ Stockage : un fichier Markdown + TOML par tâche, médias (sha256)
crates/adapters/note_markdown/ Conversion note Markdown ⇄ document TipTap (ADR-0007), texte cherchable des notes
crates/adapters/memory_index/ Index de recherche en mémoire (port SearchIndex)
crates/adapters/multi_base/  Plusieurs bases vues comme une (composites, ADR-0009)
crates/adapters/crypto/      Coffre : Argon2id + XChaCha20-Poly1305 (ports VaultKeys, Cipher — ADR-0010)
crates/adapters/keyring/     Trousseau de l'OS (port KeyringStore : « Se souvenir sur cet appareil »)
src-tauri/                   Composition root (composition.rs), commandes, DTO, événements, protocole tq-media://
ui/src/domain/               Types TS miroirs des DTO
ui/src/ports/                Interfaces TaskGateway, MediaGateway, VaultGateway… (seul accès au cœur)
ui/src/infrastructure/       tauri/ (invoke) et in-memory/ (navigateur, tests)
ui/src/application/          Composables (useTaskTree, useStatusFlow…) : orchestration d'affichage
ui/src/presentation/         Composants Vue ; editor/ = éditeur TipTap et ses vues de nœuds
docs/                        Spec, architecture, guides, ADR
poc/                         POC I0 (archivés, ne pas faire évoluer)
```

Règle de dépendance (vérifiée par Cargo : une crate ne peut pas importer ce qu'elle ne déclare pas) :
`domain` ← `application` ← `adapters` ← `src-tauri`. Côté front : `presentation` → `application` → `ports` ← `infrastructure`.

Retrouver une notion Rust expliquée dans le code : `grep -rn "\[rust\]" crates src-tauri`.

---

## 5. Parcours d'une action de bout en bout

Exemple : **clore une tâche** (`Espace`), règles R1 et R2.

| # | Où | Rôle |
|---|---|---|
| 1 | `ui/src/application/keyboard.ts` | `Espace` → action `toggleDone` |
| 2 | `ui/src/application/useStatusFlow.ts` → `close()` | Demande l'impact (`getCloseImpact`) ; s'il reste des sous-tâches ouvertes, ouvre la boîte de confirmation (R2) |
| 3 | `ui/src/ports/TaskGateway.ts` → `closeTask()` | Le contrat ; le composable ne connaît que lui |
| 4 | `ui/src/infrastructure/tauri/TauriTaskGateway.ts` | `invoke('close_task', { taskId, scope })` |
| 5 | `src-tauri/src/commands/status.rs` → `close_task` | Traduit le DTO (`parse_task_id`, `CloseScopeDto → CloseScope`) et lance le cas d'usage dans un fil (`run`, `spawn_blocking`) |
| 6 | `crates/application/src/use_cases/close_task.rs` | Charge l'arbre (`TreeSession`), horodate (`Stamper`), clôt, calcule la proposition R1, enregistre, publie l'événement |
| 7 | `crates/domain/src/task.rs` → `Task::close` ; `rules/closure.rs` | **La décision** : déjà close ? dans la corbeille ? quel parent proposer ? |
| 8 | `crates/adapters/fs_markdown/src/storage.rs` | Écriture atomique du fichier de la tâche |
| 9 | `src-tauri/src/events.rs` | Émet `tasks-changed` vers toutes les fenêtres |
| 10 | `useTaskTree.ts` (`onTasksChanged`) ; `useStatusFlow.ts` | L'arbre se rafraîchit ; si `proposeClosingParent`, nouvelle confirmation (R1) |

L'assemblage (qui reçoit quel port) est fait **une seule fois** dans `src-tauri/src/composition.rs`.

**Mise en forme des listes** (spec §5.3) : `list_tasks` renvoie les tâches **et** une `ListLayout`
par liste (vue effective, origine, enfants filtrés et triés), calculées par `tq_domain::view`.
L'UI (`treeView.ts`) se contente de suivre `children` ; l'éditeur `ViewMenu.vue` appelle
`customize_list_view` / `inherit_list_view`.

**Variante lecture seule — rechercher** (`Mod+F`) : `SearchBox.vue` → `useSearch.ts` (pause de
frappe 120 ms) → `TaskGateway.searchTasks` → `invoke('search_tasks')` → `commands/search.rs` →
`SearchTasks` → port `SearchIndex` (`memory_index`) → `tq_domain::search::search_documents`.
L'index n'est jamais rempli par la recherche : il suit le dépôt grâce au décorateur
`IndexingTaskRepository` (§6 de l'archi).

---

## 6. Recettes : faire évoluer le code

### Ajouter un cas d'usage
1. **Domaine d'abord** si une règle est nouvelle : fonction pure dans `crates/domain/src/rules/` + tests unitaires.
2. Fichier `crates/application/src/use_cases/<verbe_objet>.rs` : struct avec ses ports en `Arc<dyn …>`, `new`, `execute(&self, command) -> Result<…, AppError>`. Le déclarer dans `use_cases/mod.rs`.
3. Tests dans `crates/application/tests/use_cases.rs` avec `crates/testing` (`TestApp`, `TaskBuilder`).
4. Brancher dans `src-tauri/src/composition.rs` : champ dans `AppState` + construction.

### Ajouter une couleur de texte (ADR-0012)
Quatre endroits, tenus ensemble : `TEXT_COLORS` en Rust (`crates/adapters/note_markdown/src/formatting.rs`)
et en TypeScript (`ui/src/presentation/editor/formatting.ts`), les variables `--text-<nom>` des
deux thèmes (`ui/src/presentation/styles.css`, plus les règles `span[data-color=…]` de
`NoteEditor.vue` et `FormatMenus.vue`), le libellé `format.<nom>` (`ui/src/i18n/fr.ts`). Un nom
inconnu du Rust n'est jamais écrit dans le fichier.

### Ajouter une icône
Une entrée dans `ICONS` de `ui/src/presentation/AppIcon.vue` : le contenu d'un `<svg viewBox="0 0 24 24">`
dessiné au trait (`stroke`, pas de `fill` sauf pastille), aligné sur la grille 24 × 24, comme les
autres. Puis `<AppIcon name="…" />`. Pas d'emoji dans l'interface : leur rendu change d'un système à
l'autre.

### Exposer une commande à l'interface
1. `src-tauri/src/commands/<famille>.rs` : fonction `#[tauri::command] async`, **mince** (DTO → commande → `run(...)` → DTO).
2. DTO dans `src-tauri/src/dto.rs` (`#[serde(rename_all = "camelCase")]`).
3. Enregistrer dans `generate_handler![…]` (`src-tauri/src/lib.rs`).
4. Front : méthode dans le port (`ui/src/ports`), implémentations **Tauri et en mémoire** (`ui/src/infrastructure`), types dans `ui/src/domain`.
5. Commande pour une nouvelle fenêtre ou permission : vérifier `src-tauri/capabilities/default.json`.

### Ajouter un port et son adaptateur
1. Trait dans `crates/application/src/ports/` (petit, une responsabilité), avec son type d'erreur.
2. Fake dans `crates/testing/src/fakes.rs` ; si le comportement doit être identique, une fonction de **contrat** dans `crates/testing/src/contract.rs`, appelée par le fake et par l'adaptateur.
3. Adaptateur dans `crates/adapters/<nom>/` (nouvelle crate si nouvelle technologie), qui traduit ses erreurs techniques en erreurs du port.
4. Assemblage dans `composition.rs`.

### Ajouter une dépendance (crate ou paquet npm)
Politique : ADR-0006 et `02-architecture.md` §16. Se demander d'abord si < ~50 lignes testées suffisent
ou si Tauri / la plateforme web le fournit. Sinon : l'ajouter à l'inventaire §16 **avec sa justification**,
obtenir l'accord du mainteneur, puis seulement installer.

### Décision structurante
Nouvel ADR dans `docs/adr/` (numéro suivant, format des existants), référencé dans `docs/adr/README.md` et `02-architecture.md`.

---

## 7. Déboguer

| Côté | Moyen |
|---|---|
| Front (webview) | Dans l'app lancée par `pnpm dev` : clic droit → **Inspecter** (outils de développement WebKit sous macOS, Edge sous Windows). Console, réseau (`tq-media://`), Vue. |
| Front sans Rust | `pnpm ui:dev` dans un navigateur + Vue DevTools |
| Rust | Le terminal de `pnpm dev` affiche les sorties et les paniques. `RUST_BACKTRACE=1 pnpm dev` pour la pile complète. |
| Rust pas à pas | RustRover / VS Code (CodeLLDB) : déboguer un test (`cargo test`) est le plus simple ; pour l'app, attacher le débogueur au processus `todoquoi` |
| Erreurs reçues par l'UI | Toujours un `UiError { code, message }` (`src-tauri/src/error.rs`) ; le `code` permet de retrouver la variante d'origine |
| Données | Ouvrir les fichiers `.md` du dossier de données ; ils sont lisibles et modifiables à la main |

---

## 8. Définition de « terminé » et flux Git

Une tâche ou un incrément est terminé quand :
- [ ] les vérifications du §3 passent (fmt, clippy, tests Rust ; lint, tests front) ;
- [ ] chaque fichier Rust nouveau a son `//!`, chaque élément public son `///`, et les notions Rust nouvelles leur `// [rust]` ;
- [ ] la spec (`01`), l'architecture (`02`), un ADR si décision structurante, et **ce guide** (`04`) et le guide Rust (`03`) sont à jour ;
- [ ] la fonctionnalité a été essayée dans l'app réelle (`pnpm dev`), pas seulement en tests.

Flux Git : une branche par incrément (`feat/i<n>-<sujet>`) ou par sujet (`docs/…`, `fix/…`) ;
messages au format `type(portée): résumé` en français ; fusion locale **en avance rapide** dans `main` ;
pas de PR. Pas de secret ni de donnée réelle dans le dépôt (fixtures fictives uniquement).

---

## 9. Problèmes connus

| Symptôme | Cause / solution |
|---|---|
| Glisser-déposer de fichier inopérant dans l'éditeur | Tauri intercepte les fichiers déposés par défaut : désactivé par `"dragDropEnabled": false` (fenêtre principale) et `.disable_drag_drop_handler()` (fenêtres de tâche). À conserver pour toute nouvelle fenêtre. |
| Image / audio ne s'affiche pas | Vérifier la CSP de `src-tauri/tauri.conf.json` (`img-src` / `media-src` doivent inclure `tq-media:` et `http://tq-media.localhost` pour Windows). |
| macOS : le micro ne fonctionne pas | La clé `NSMicrophoneUsageDescription` de `src-tauri/Info.plist` doit être présente ; autoriser l'app dans Réglages → Confidentialité. |
| macOS : accès refusé à `~/Documents` ou au drive depuis un outil en ligne de commande | Protection TCC : autoriser le terminal dans Réglages → Confidentialité → Fichiers et dossiers. |
| Dossier du drive qui ne répond pas (fichiers « à la demande ») | Prévu : toute E/S sur le drive a un délai maximal ; scan périodique (I5). |
| `vue-tsc` échoue après mise à jour de TypeScript | `vue-tsc` n'accepte pas encore TypeScript 7 : TypeScript reste épinglé en `^6`. |
| Windows : fichiers modifiés sans changement visible | Fins de ligne : vérifier `core.autocrlf=false` ; le dépôt est en LF. |
| Clic sur un lien de note sans effet | Voulu : la webview ne navigue jamais (elle quitterait l'app). `Mod+clic` ou « Ouvrir » passe par `open_link`, qui refuse tout sauf `http`, `https`, `mailto`. |
| Glisser une tâche ne fait rien | Les repères n'apparaissent que là où le dépôt est possible (pas dans ses propres sous-tâches). Le cœur revérifie (R11). |
| Indicateur rouge « Drive inaccessible » | Le dossier ne répond pas dans les 15 s (drive hors ligne, premier accès lent) : nouvel essai toutes les 3 s, rien n'est perdu. |
| Déverrouiller un coffre prend plusieurs secondes en développement | Argon2id (64 Mio, 3 passes) non optimisé : `argon2` et `blake2` sont compilés en `opt-level = 3` même en `dev` (`Cargo.toml` racine, `[profile.dev.package.*]`). Si c'est lent, vérifier que ces lignes sont toujours là. |
| macOS : le Trousseau demande l'autorisation à chaque relance | L'entrée est liée à la signature du binaire, qui change à chaque build de développement. Depuis le correctif, **une build de développement n'utilise plus le trousseau** (« Se souvenir » y est sans effet). Pour l'essayer : `TODOQUOI_KEYCHAIN=1 pnpm dev` (macOS) / `$env:TODOQUOI_KEYCHAIN="1"; pnpm dev` (Windows), en acceptant les demandes. Une entrée créée avant reste dans le trousseau : la supprimer dans Trousseaux d'accès (service `TodoQuoi`). |
| La note d'une tâche reste verrouillée après déverrouillage | Elle a été chiffrée par un **autre** coffre (`key_id` du fichier ≠ celui du manifeste, cf. ADR-0010) ou son texte chiffré est abîmé : elle reste scellée, rien n'est écrasé. |
| Une image d'une note chiffrée ne s'affiche pas (404 `tq-media`) | Le coffre qui l'a chiffrée est fermé, ou c'est un autre coffre que celui de la base (`key_id`, ADR-0010). Déverrouiller la base ; en dernier recours, le fichier `.tqe` est intact dans `media/`. |
| « Révéler dans le Finder » refusé sur une pièce jointe | Voulu pour une note chiffrée : le fichier sur disque est chiffré (`.tqe`), rien de lisible à montrer. « Ouvrir » passe par une copie déchiffrée temporaire. |
| Linux : « Se souvenir sur cet appareil » sans effet | Voulu jusqu'en I7 : pas de trousseau géré, le coffre fonctionne. |
| Windows : `Une stratégie de contrôle d'application a bloqué ce fichier. (os error 4551)` pendant `cargo build` / `cargo test` / `pnpm dev`, au hasard | **Smart App Control** (Windows 11) bloque des exécutables fraîchement compilés et non signés (scripts `build-script-build.exe`, binaires de test). Ce n'est pas le code : relancer la commande. Vérifier : Observateur d'événements → `Microsoft/Windows/CodeIntegrity/Operational`, événement 3077. Le désactiver (Sécurité Windows → Contrôle des applications et du navigateur) est un choix système, longtemps irréversible sans réinstaller Windows. |
| Windows : fenêtre noire quelques secondes au premier lancement d'un binaire release | Vérification de l'exécutable neuf (Smart App Control) et démarrage de WebView2 : l'interface apparaît ensuite. |
| Windows : premier `pnpm build` long, puis `target/release/bundle/msi/` | La cible `msi` utilise WiX 3, que Tauri **télécharge** au premier build dans `%LOCALAPPDATA%\tauri\WixTools314` (nécessite VBScript, fonctionnalité Windows facultative). Sans WiX : `pnpm tauri build --no-bundle`. |
| Tests de `drive_folders` qui trouvent un vrai drive | `detect_in` reçoit le dossier personnel, les variables et les lecteurs Google Drive (`G:\…`) en paramètres : un test ne doit jamais dépendre des disques de la machine. |
| Simuler un autre appareil en développement | Modifier à la main un fichier de `tasks/` (ou en copier un en `<id> (1).md`) pendant que l'app tourne : la liste se met à jour en ≤ 3 s, la copie est réglée et rangée dans `.conflicts/`. |
