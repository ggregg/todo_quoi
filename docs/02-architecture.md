# TodoQuoi — Architecture technique (V1)

> Statut : **brouillon à valider** · Version 0.3 · 2026-11-06
> v0.2 : architecture **hexagonale**, SOLID / Clean Code, **code pédagogique**.
> v0.3 : **audit des dépendances** (§16) — front-matter TOML, audio et images traités dans la webview, index en mémoire, scan périodique au lieu d'un watcher.

## 0. Exigences de conception (non négociables)

1. **Code pédagogique** : un développeur senior TypeScript/Vue, débutant en Rust, doit pouvoir lire, comprendre et reprendre chaque fichier. Voir §9 et [`03-guide-rust-pour-dev-ts.md`](03-guide-rust-pour-dev-ts.md).
2. **Architecture hexagonale** (ports & adaptateurs) / Clean Architecture : le métier ne dépend de rien de technique.
3. **SOLID** et **Clean Code**.
4. **Testabilité** : métier et cas d'usage testables sans disque, sans réseau, sans Tauri.
5. **Sobriété des dépendances** : une dépendance n'entre que si elle apporte ce qu'on ne peut pas écrire simplement et sûrement soi-même (§16).

## 1. Vue d'ensemble

```
                         ┌──────────────── Adaptateurs PRIMAIRES (pilotants) ────────────────┐
                         │  Tauri commands (IPC depuis l'UI Vue)   Scanner du dossier drive   │
                         └───────────────────────────────┬───────────────────────────────────┘
                                                         │ appellent
                     ┌───────────────────────────────────▼───────────────────────────────────┐
                     │                     APPLICATION  (cas d'usage)                         │
                     │  CreateTask · CloseTask · ReopenTask · MoveTask · DeleteTask · Restore │
                     │  EditNote · StoreMedia · SearchTasks · EncryptTask · ApplyExternal…    │
                     │                                                                        │
                     │  PORTS (traits) : TaskRepository · MediaStore · SearchIndex · Cipher   │
                     │  KeyVault · Clock · IdGenerator · FileHydrator · EventPublisher        │
                     │            ┌────────────────────────────────────────────┐              │
                     │            │              DOMAINE (cœur pur)             │              │
                     │            │  Task · TaskId · Tag · Note · Block ·       │              │
                     │            │  TaskTree · ClosurePolicy · ConflictPolicy  │              │
                     │            │  aucune E/S, aucune dépendance technique    │              │
                     │            └────────────────────────────────────────────┘              │
                     └───────────────────────────────────▲───────────────────────────────────┘
                                                         │ implémentent les ports
       ┌─────────────────────────── Adaptateurs SECONDAIRES (pilotés) ──────────────────────────┐
       │  fs_markdown (drive)   memory_index   crypto   keyring   cloud_sync (OS)   system       │
       └─────────────────────────────────────────────────────────────────────────────────────────┘
```

**Règle de dépendance** : les flèches de dépendance pointent **toujours vers le centre**.
`domaine` ← `application` ← `adaptateurs` ← `composition root` (binaire Tauri).
Imposée **par le compilateur** : chaque couche est une crate Cargo ; `domain` n'a pas `tauri` dans ses dépendances, impossible de l'importer par erreur.

**Principe de données** : le **dossier du drive est la source de vérité** ; le cache local est jetable et reconstructible. Aucun serveur, aucun OAuth en V1.

### 1.1 Développement vs exécution — Node.js n'est **jamais** embarqué

| | Développement (`pnpm tauri dev`) | Build (`pnpm tauri build`) | Exécution (app installée) |
|---|---|---|---|
| Node.js | **Oui** : Vite dev server `localhost:5173` (HMR) | **Oui** : `vite build` → `ui/dist/` | **Non** |
| Rust | Debug, la webview pointe sur Vite | Release, `ui/dist/` **embarqué dans le binaire** | Le binaire seul |
| Service de la SPA | Vite | — | **L'app elle-même**, protocole interne (`tauri://localhost` macOS/Linux, `http://tauri.localhost` intercepté sous Windows). **Aucun serveur HTTP, aucun port.** |
| Webview | WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux) | idem | idem — pas de Chromium embarqué |

En exécution : un **processus Rust natif** ouvre des fenêtres système contenant une webview ; la SPA y est chargée depuis les ressources embarquées et dialogue avec le Rust par **IPC** (`invoke` / événements). Les médias passent par le protocole `tq-media://` servi par le même processus.

L'UI peut aussi tourner **seule dans un navigateur** (`pnpm dev`) avec une passerelle en mémoire (§8).

## 2. Les couches en détail

### 2.1 Domaine — `crates/domain` (`tq_domain`)

| Élément | Rôle |
|---|---|
| `TaskId`, `Tag`, `DeviceId`, `MediaRef` | *Newtypes* validés à la construction. |
| `Timestamp` | Millisecondes depuis l'époque Unix (`i64`) + conversion RFC 3339 **écrite à la main** et testée. |
| `Task` | Entité. Méthodes intention-révélatrices : `close(at)`, `reopen(at)`, `rename(..)`, `add_tag(..)`. |
| `TaskStatus` | `enum { Open, Done { at } }`. |
| `Note`, `Block` | `Note { blocks: Vec<Block> }` ; `enum Block { Paragraph, Heading, BulletList, OrderedList, Checklist, RadioGroup, Table, Code, Quote, Image, Audio, Attachment, Rule }`. Le domaine ignore le Markdown. |
| `TaskTree` | Enfants, ancêtres, descendants, détection de cycles (R11). |
| `ClosurePolicy` | R1–R4, fonctions pures (une proposition **par niveau**). |
| `ConflictPolicy` | « Dernier qui écrit gagne » : horodatage hybride, puis `DeviceId`. |
| `OrderKey` | Indice fractionnaire (`between(a, b)`), écrit à la main. |
| `DomainError` | Erreurs métier typées. |

Dépendances : `thiserror`, `serde`. **Rien d'autre.**

### 2.2 Application — `crates/application` (`tq_application`)

```rust
// crates/application/src/use_cases/close_task.rs  (esquisse)

/// Cas d'usage : clôturer une tâche.
/// Renvoie la proposition éventuelle de clôturer aussi le parent (règle R1, un niveau à la fois).
pub struct CloseTask {
    tasks: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventPublisher>,
}

impl CloseTask {
    pub fn execute(&self, command: CloseTaskCommand) -> Result<CloseTaskOutcome, AppError> {
        let mut tree = self.tasks.load_tree()?;
        let now = self.clock.now();

        let task = tree.get_mut(&command.task_id)?;
        task.close(now)?;
        self.tasks.save(task)?;

        let proposal = ClosurePolicy::parent_to_propose(&tree, &command.task_id);
        self.events.publish(DomainEvent::TaskChanged(command.task_id));
        Ok(CloseTaskOutcome { proposal })
    }
}
```

**Ports secondaires** (petits, ISP) :

| Port | Responsabilité unique |
|---|---|
| `TaskRepository` | Charger / enregistrer / supprimer des tâches. |
| `MediaStore` | Stocker / lire un média (octets déjà préparés par le front) par son empreinte. |
| `SearchIndex` | Indexer une tâche, rechercher. |
| `Cipher` | Chiffrer / déchiffrer des octets. |
| `KeyVault` | Mémoriser / retrouver la clé du coffre. |
| `Clock` | Donner l'heure. |
| `IdGenerator` | Générer des `TaskId`. |
| `FileHydrator` | Forcer le téléchargement d'un fichier « à la demande ». |
| `EventPublisher` | Notifier l'UI d'un changement. |

Plus de ports `ImageOptimizer` / `AudioRecorder` : capture audio et compression d'images sont faites **dans la webview** (§7), le Rust ne reçoit que des octets prêts à stocker.

**Choix pédagogiques** : domaine et application **synchrones** (ADR-0002) ; injection par `Arc<dyn Trait>` (ADR-0003).

### 2.3 Adaptateurs secondaires — `crates/adapters/*`

| Crate | Implémente | Dépendances externes |
|---|---|---|
| `fs_markdown` | `TaskRepository`, `MediaStore` | `pulldown-cmark`, `toml`, `sha2` |
| `memory_index` | `SearchIndex` (+ instantané JSON du cache, état d'UI local) | aucune (`serde_json` déjà présent) |
| `crypto` | `Cipher` | `argon2`, `chacha20poly1305`, `hmac`, `zeroize` |
| `keyring` | `KeyVault` | `keyring` |
| `cloud_sync` | `FileHydrator` | `windows` (Windows) / `objc2-foundation` (macOS), no-op Linux |
| `system` | `Clock` (horloge hybride), `IdGenerator` | aucune (aléa via `getrandom`, déjà tiré par la crypto) |

### 2.4 Adaptateurs primaires — `src-tauri`
- **Commandes Tauri** : DTO → commande applicative → cas d'usage → DTO. Aucune règle métier.
- **Scanner du dossier drive** : thread qui, toutes les **3 s** (et au réveil / retour au premier plan), compare taille + date de modification des fichiers au cache et appelle `ApplyExternalChange` pour ceux qui ont changé. Pas de dépendance de surveillance du système de fichiers (§5.2).
- **Composition root** (`src-tauri/src/composition.rs`) : seul endroit qui connaît les implémentations concrètes.
- Erreurs : un type `UiError { code, message }` sérialisable, sans `anyhow`.

### 2.5 Kit de test — `crates/testing` (*dev-dependency*)
Fakes en mémoire de chaque port, tests de contrat partagés fake / adaptateur réel (Liskov), constructeurs de données de test. Dossiers temporaires via `std::env::temp_dir()` + nettoyage, sans crate dédiée.

## 3. SOLID et Clean Code appliqués

| Principe | Application concrète |
|---|---|
| **S** | Un cas d'usage par fichier ; un port = une responsabilité ; format Markdown (`fs_markdown::format`) séparé de l'accès disque (`fs_markdown::storage`). |
| **O** | Ajouter un drive par API (V2) = nouvel adaptateur ; nouveau bloc = nouvelle variante de `Block`, le compilateur liste les `match` à compléter. |
| **L** | Tests de contrat partagés entre fake et implémentation réelle. |
| **I** | Ports petits ; un cas d'usage ne reçoit que les ports dont il se sert. |
| **D** | Les cas d'usage dépendent de traits ; seule la composition root connaît les types concrets. |

Clean Code : fonctions < 25 lignes visé, retours anticipés, noms révélant l'intention, pas de paramètre booléen, types plutôt que commentaires, pas d'`unwrap()` hors tests, aucune règle métier dupliquée dans le front.

## 4. Stockage sur le drive (adaptateur `fs_markdown`)

```
TodoQuoi/
├── manifest.json            # schéma, id d'espace, paramètres du coffre, vue racine
├── tasks/
│   ├── 0195f3c2-…-7b1e.md    # tâche en clair (nom = UUID v7, triable par date)
│   └── 0195f3c4-…-91aa.tqe   # tâche chiffrée
├── media/
│   ├── 3f9a…c2.jpg           # nom = SHA-256 du contenu (immuable)
│   ├── 81bd…0e.webm          # audio WebM/Opus (format unique, tous OS)
│   └── 5c11…aa.tqe           # média d'une tâche chiffrée
├── .devices/
│   └── mac-greg.json         # dernier passage, version app, version schéma
└── .conflicts/
    └── 0195f3c2-…-7b1e.2026-11-05T17-25-12Z.mac-greg.md
```

Arborescence plate, ordre par indice fractionnaire, médias immuables adressés par hash, corbeille par champ `deleted_at` (inchangé).

**Identifiants** : UUID v7 (48 bits d'horodatage + 74 bits aléatoires, format standard RFC 9562) **générés à la main** dans `system` (~30 lignes testées) — triables chronologiquement comme les ULID, sans dépendance.

### 4.1 Format d'une tâche — front-matter **TOML** entre `+++`

```markdown
+++
schema_version = 1
id = "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e"
parent = "0195f3c0-1a2b-7c3d-8e4f-5a6b7c8d9e0f"
order = "a0V"
title = "Préparer la release 1.2"
done = false
tags = ["release", "client-x"]
created_at = 2026-11-05T16:02:11.120Z
updated_at = 2026-11-05T17:25:12.345Z
updated_by = "mac-greg"
encrypted = false

[view]
columns = ["title", "progress", "tags", "updated_at"]
sort = [{ field = "manual" }]
filter = { status = "open", tags_in = [], tags_out = [] }
+++
Checklist de la release :

- [x] Changelog
- [ ] Tag Git

:::radio{label="Cible"}
- ( ) Recette
- (x) Production
:::

![capture](../media/3f9a…c2.jpg){width=480}

::audio{src="../media/81bd…0e.webm" duration=12.4}
```

- Champs absents = valeur par défaut (`parent` absent → racine, `done_at` / `deleted_at` absents → `None`) : TOML n'a pas de `null`, et le fichier reste court.
- `+++` est la convention TOML de Hugo / Zola ; les éditeurs Markdown l'affichent comme du texte.
- `fs_markdown::format` : `Note` ↔ Markdown étendu (`pulldown-cmark` + directives `:::radio`, `::audio`, `::file` analysées à la main). Test : `parse(render(note)) == note` sur un corpus.
- Champs inconnus préservés (compatibilité avant) ; `fs_markdown::migrations` : une fonction par saut de version.

### 4.2 Tâche chiffrée (`.tqe`)
```
"TQE1" | u32 len | en-tête JSON en clair | nonce 24 o | texte chiffré (+ tag Poly1305)
```
En-tête en clair minimal (`schema_version, id, parent, order, done, updated_at, updated_by, deleted_at, key_id`), AAD = en-tête, médias chiffrés nommés par HMAC.

## 5. Synchronisation

### 5.1 Écriture
1. Sérialiser → `tasks/.~<id>.tmp` → `fsync` → `rename` atomique.
2. `updated_at` = horloge hybride (`system::HybridClock`).
3. Débounce 1 s par tâche ; flush à la fermeture.

### 5.2 Changements externes — scan périodique
- Toutes les **3 s** : lister `tasks/` et comparer `(taille, date de modification)` à l'instantané en mémoire ; ne relire que les fichiers changés. Coût : quelques millisecondes pour quelques milliers de fichiers.
- Également au retour au premier plan et au réveil de la machine.
- Pourquoi pas un watcher (`notify`) : les dossiers des drives modernes (File Provider macOS, Cloud Files Windows) perdent des événements ; il fallait de toute façon un scan de réconciliation. Le scan seul est plus simple, plus prévisible, et supprime une dépendance.
- Fichier changé → `ApplyExternalChange` → `ConflictPolicy` → index → événement UI.
- `stat()` ne rapatrie pas un fichier « à la demande » (vérifié au POC I0-3, iCloud + Google Drive) : le scan reste léger même quand le drive a évincé des fichiers.
- **Toute E/S sur le drive a un délai maximal** et tourne hors du fil de l'UI : au POC, le premier accès au dossier Google Drive est resté bloqué plus de 2 min. Délai dépassé → indicateur « drive inaccessible », nouvel essai au scan suivant.

### 5.3 Conflits
| Situation | Traitement |
|---|---|
| Modifié sur disque alors qu'on a des modifs locales non écrites | Le plus récent gagne ; perdant → `.conflicts/`. Tâche ouverte : bandeau Recharger / Garder la mienne. |
| Copie de conflit du drive (`x (conflicted copy).md`, `x-DESKTOP-ABC.md`, `x (1).md`…) | Détection par motifs + `id` identique : idem. |
| Fichier illisible | Ignoré, signalé ⚠, jamais réécrit. |

### 5.4 Aides OS (`cloud_sync`, port `FileHydrator`)
| OS | Problème | Solution |
|---|---|---|
| Windows (Cloud Files API) | Fichiers « à la demande » | Attributs `RECALL_ON_*`, `CfHydratePlaceholder`, épinglage proposé. **Non encore vérifié** (POC I0-3, volet Windows). |
| macOS — iCloud **et** File Provider (Google Drive, OneDrive, Dropbox) | Fichiers évincés / non matérialisés | **Validé au POC I0-3** : détection par le drapeau `SF_DATALESS` de `stat()` (bibliothèque standard, sans Objective-C) ; préchargement par `NSFileManager.startDownloadingUbiquitousItem` (~1 s pour 200 Ko) ; état par `NSURLUbiquitousItemDownloadingStatusKey`. Une lecture simple rapatrie aussi le fichier, mais en bloquant : on précharge plutôt. |
| Linux | — | no-op. |

## 6. Cache local et recherche (`memory_index`)
- Emplacement : `app_local_data_dir()` fourni par Tauri (`%LOCALAPPDATA%\…`, `~/Library/Application Support/…`, `$XDG_DATA_HOME/…`).
- **Index en mémoire** : pour chaque tâche, texte « à plat » normalisé (minuscules, accents retirés, à la main via une table de correspondance Unicode Latin) ; recherche = filtres structurés (`tag:`, `is:`, `has:`, `in:`) + sous-chaînes. Cible : < 10 ms pour 10 000 tâches.
- **Instantané** `cache.json` (projection des tâches + `taille/date` des fichiers) écrit à la fermeture et périodiquement → démarrage rapide sans relire tous les fichiers. Perdu ou incompatible → reconstruction depuis le drive.
- `ui_state.json` : dépliage, fenêtres (local à l'appareil).
- Tâches chiffrées : présentes dans l'index **seulement** coffre déverrouillé, jamais dans `cache.json`.
- Si le volume dépasse un jour ~50 000 tâches : réévaluer (SQLite + FTS5) — le port `SearchIndex` permet de changer d'implémentation sans toucher au reste.

## 7. Crypto et médias
- **Crypto** : Argon2id (m = 64 Mio, t = 3, p = 1), XChaCha20-Poly1305, clé de contenu enveloppée, `zeroize`.
- **Images — dans la webview** : `createImageBitmap` (applique l'orientation EXIF) → canvas redimensionné (côté long ≤ 2560 px) → `toBlob` en **JPEG qualité 85**, ou **PNG** si l'image a de la transparence ; GIF animés et SVG conservés tels quels. Le réencodage supprime les métadonnées EXIF. Le Rust reçoit les octets finaux.
- **Audio — dans la webview** : `MediaRecorder` **imposé en WebM/Opus** (`mimeType: 'audio/webm;codecs=opus'`, `audioBitsPerSecond: 32000`) — enregistré et lu par WebView2 **et** WKWebView (POC I0-2, `poc/i0-2-medias/RESULTATS.md`). Durée mesurée pendant l'enregistrement et stockée dans `::audio{duration=…}`. Autorisation micro : `NSMicrophoneUsageDescription` (macOS), invite WebView2 (Windows).
- **Images** : compression dans un Worker (`OffscreenCanvas`) pour ne pas figer l'interface (jusqu'à 3 s observées sur le fil principal).
- Médias servis au front par `tq-media://<hash>` (déchiffrement à la volée pour les tâches chiffrées).

## 8. Front (Vue 3)

```
ui/src/
├── domain/          # types TS des DTO, écrits à la main + fixtures JSON partagées avec Rust
├── application/     # composables (état réactif de module) : sélection, fenêtres, filtres
├── ports/           # interfaces TaskGateway, MediaGateway…
├── infrastructure/
│   ├── tauri/       # implémentations via invoke()/listen()
│   └── in-memory/   # données fictives : Vitest et `pnpm dev` dans un navigateur
├── i18n/            # dictionnaire typé fr.ts + fonction t()
└── presentation/    # composants Vue, éditeur TipTap et extensions maison
```

| Besoin | Choix | Dépendance |
|---|---|---|
| Build | Vite + TypeScript strict | `vite`, `@vitejs/plugin-vue`, `typescript`, `vue-tsc` |
| État | Composables + `reactive()` de module (pas de Pinia) | — |
| Arbre virtualisé | Composant maison (hauteur de ligne fixe, ~40 lignes) | — |
| Glisser-déposer | HTML5 natif | — |
| Éditeur | TipTap + extensions maison `RadioGroup`, `AudioBlock`, `FileBlock`, `TaskLink` | `@tiptap/*` |
| i18n | Dictionnaire typé, clés vérifiées par le compilateur | — |
| Coloration syntaxique | **Reportée après V1** (chasse fixe) | — |
| Boîtes de dialogue, ouverture de fichiers | Via nos commandes Rust | — |
| Fenêtres de tâche | `WebviewWindow`, route `/task/:id` | `@tauri-apps/api` |

## 9. Code pédagogique — conventions

1. En-tête `//!` dans chaque fichier : rôle + place dans l'hexagone.
2. `///` sur tout élément public (quoi, pourquoi, erreurs, exemple si utile).
3. `// [rust]` à la première occurrence d'un concept dans un fichier, avec l'équivalent TS (`rg "\[rust\]"`).
4. Sous-ensemble simple : structs propriétaires, pas de durées de vie explicites sauf nécessité documentée, pas de macros maison, `Arc<dyn Port>`, `Mutex` seulement dans les adaptateurs, erreurs avec `thiserror`, pas d'`unsafe` hors `cloud_sync` (avec `// SAFETY:`).
5. Identifiants en anglais, commentaires et docs en français.
6. ADR pour toute décision structurante (`docs/adr/`).
7. `rustfmt`, `clippy -D warnings` (+ `clippy::unwrap_used`, `clippy::too_many_lines`), `cargo doc --open`.

## 10. Organisation du dépôt

```
TodoQuoi/
├── Cargo.toml                       # workspace
├── crates/
│   ├── domain/
│   ├── application/
│   ├── adapters/
│   │   ├── fs_markdown/
│   │   ├── memory_index/
│   │   ├── crypto/
│   │   ├── keyring/
│   │   ├── cloud_sync/
│   │   └── system/
│   └── testing/
├── src-tauri/
│   └── src/ (main.rs, composition.rs, commands/, dto/, scanner.rs)
├── ui/
├── docs/ (01, 02, 03, adr/)
├── maquette/index.html
├── poc/                             # POC I0, jetables
├── CLAUDE.md
├── .gitattributes · .gitignore · .editorconfig
```

## 11. Stratégie de test

| Couche | Type | Outils |
|---|---|---|
| Domaine | Unitaires, tables de cas (R1–R12, arbre, ordre, conflits, UUID v7, RFC 3339) | `cargo test` |
| Application | Cas d'usage avec fakes | `tq_testing` |
| Adaptateurs | Contrat + intégration (dossier temporaire, deux « appareils » concurrents, copies de conflit simulées) | `cargo test` |
| Format | Aller-retour Markdown sur corpus, migrations | fichiers d'exemple |
| Contrat IPC | Fixtures JSON partagées, désérialisées par Rust et typées côté TS | `cargo test` + Vitest |
| Front | Composants et composables avec la passerelle en mémoire | Vitest |
| Bout en bout | Parcours manuels documentés en V1 ; automatisation après V1 | — |

CI : `fmt --check`, `clippy -D warnings`, `test`, `doc`, `vue-tsc`, `vitest` sur Windows + macOS.

## 12. Plan par incréments

| # | Contenu | Livrable |
|---|---|---|
| **I0** | **POC** : (1) aller-retour TipTap ↔ Markdown étendu + TOML ; (2) `MediaRecorder` + lecture croisée, compression d'image par canvas — ✔ GO (WebM/Opus unique) ; (3) hydratation — ✔ GO macOS (iCloud + Google Drive), OneDrive/Windows à vérifier | **GO** (`poc/README.md`) |
| I1 | Workspace, `domain`, `application` (CRUD), `testing`, `fs_markdown` (sans scanner), squelette Tauri + Vue, arbre, clavier | Tâches dans un dossier local |
| I2 | Panneau + fenêtres de tâche, éditeur (texte, liens, cases, tableaux, code), tags | Notes riches sans médias |
| I3 | Médias (images, pièces jointes, audio), radios | Notes complètes |
| I4 | `memory_index` : recherche, vues par liste + héritage, zoom | Recherche et présentation |
| I5 | Scanner, conflits, `cloud_sync`, indicateur, corbeille | Usage sur 2 appareils |
| I6 | `crypto` + `keyring` | Chiffrement par tâche |
| I7 | i18n, thèmes, accessibilité, installeurs | V1 |

Chaque incrément se termine par une **revue de lisibilité** et une **revue des dépendances** (`cargo tree --depth 1`, `pnpm why`).

## 13. Risques

| Risque | Parade |
|---|---|
| Aller-retour Markdown ↔ éditeur imparfait | Conversion unique en Rust, corpus, champs inconnus préservés |
| ~~Lecture croisée des formats audio~~ | Levé au POC I0-2 : WebM/Opus imposé, lu par les deux webviews |
| ~~`getUserMedia` / permission micro dans WKWebView via Tauri~~ | Levé au POC I0-2 (fonctionne même en `tauri dev`) |
| « Dernier qui écrit gagne » écrase une modif | `.conflicts/`, HLC, débounce court |
| Perte de la phrase de passe | Avertissement, clé de secours exportable |
| Courbe d'apprentissage Rust | Sous-ensemble simple, notes `[rust]`, guide `03`, revues |
| Sur-ingénierie | Ports et dépendances limités au strict nécessaire |

## 14. Outils de développement

| Outil | État (Mac de Greg) |
|---|---|
| Rust stable (`rustup`) | 1.99.1 ✔ |
| Node.js | 26.13 ✔ |
| pnpm (Homebrew) | 12.18.0 ✔ |
| Xcode | 27.2 ✔ |
| CLI Tauri | devDependency du projet (`pnpm tauri …`), pas d'installation globale |
| cmake | **plus nécessaire** depuis l'audit (audio dans la webview) |
| Windows | Rust MSVC + Build Tools à installer au moment du POC OneDrive (accord requis) |

## 15. ADR à rédiger en I1
- ADR-0001 — Architecture hexagonale en crates séparées.
- ADR-0002 — Domaine et application synchrones.
- ADR-0003 — Injection par `Arc<dyn Trait>`.
- ADR-0004 — Un fichier Markdown + TOML par tâche, « dernier qui écrit gagne ».
- ADR-0005 — Tauri 2 + Vue 3.
- ADR-0006 — Politique de dépendances (§16).

## 16. Politique et inventaire des dépendances

**Règle** : une dépendance entre seulement si elle (a) fait ce qu'on ne peut pas écrire simplement et sûrement (crypto, parseur, API système, éditeur riche), ou (b) n'ajoute rien au binaire (macro de compilation déjà tirée par ailleurs). Toute nouvelle dépendance est justifiée dans ce tableau **avant** d'être ajoutée.

### Rust — dépendances directes (11)
| Crate | Où | Justification |
|---|---|---|
| `tauri` | `src-tauri` | Socle |
| `serde`, `serde_json` | partout | Sérialisation ; déjà requis par Tauri |
| `thiserror` | bibliothèques | Macro de compilation, rien dans le binaire |
| `pulldown-cmark` (sans fonctionnalités par défaut) | `fs_markdown` | Parseur CommonMark/GFM : non trivial |
| `toml` | `fs_markdown` | Front-matter ; pur Rust, maintenu |
| `sha2` | `fs_markdown` | Empreinte des médias |
| `argon2`, `chacha20poly1305`, `hmac`, `zeroize` | `crypto` | On n'écrit pas de crypto |
| `keyring` | `keyring` | Trousseau OS multi-plateforme |
| `windows` / `objc2` + `objc2-foundation` | `cloud_sync` | API système (une seule par OS) ; sous macOS, `objc2-foundation` sans fonctionnalités par défaut (8 classes) |

### Écarté (écrit à la main ou fourni ailleurs)
| Écarté | Remplacé par |
|---|---|
| `anyhow` | `UiError` sérialisable |
| `directories` | `app_local_data_dir()` de Tauri |
| `ulid` / `uuid` | UUID v7 écrit à la main |
| `chrono` / `time` | `Timestamp` + RFC 3339 à la main |
| `notify` | Scan périodique |
| `rusqlite` / `tantivy` | Index en mémoire + instantané JSON |
| `image`, `webp` | Canvas dans la webview |
| `cpal`, `audiopus`, `ogg` (+ cmake) | `MediaRecorder` dans la webview |
| `tauri-specta` | DTO TS manuels + fixtures partagées |
| YAML (`serde_yaml` et forks) | TOML |
| `tempfile`, `proptest` | `std::env::temp_dir()`, tests sur corpus |

### Front — dépendances directes
| Paquet | Type | Justification |
|---|---|---|
| `vue` | runtime | Framework |
| `@tauri-apps/api` | runtime | IPC, fenêtres |
| `@tiptap/core`, `@tiptap/pm`, `@tiptap/vue-3`, `@tiptap/starter-kit`, `@tiptap/extension-image`, `@tiptap/extension-table`, `@tiptap/extension-list` | runtime | Éditeur riche (le reste est en extensions maison) |
| `vite`, `@vitejs/plugin-vue`, `typescript`, `vue-tsc` | dev | Build et typage |
| `@tauri-apps/cli` | dev | Build de l'app |
| `vitest` | dev | Tests |
| `eslint`, `prettier` (+ configs Vue/TS minimales) | dev | Qualité |

| Écarté | Remplacé par |
|---|---|
| `pinia` | Composables |
| `@tanstack/vue-virtual` | Virtualisation maison |
| `vue-draggable-plus` / SortableJS | DnD HTML5 |
| `fluent-vue` | Dictionnaire typé |
| `lowlight` / highlight.js | Reporté |
| Plugins JS Tauri (dialog, opener) | Commandes Rust |
| Playwright / `tauri-driver` | Reporté |

Les versions exactes sont figées par `Cargo.lock` et `pnpm-lock.yaml` ; la liste des **dépendances transitives** est relue à chaque incrément (`cargo tree`, `pnpm list --depth 1`).
