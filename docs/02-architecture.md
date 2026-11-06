# TodoQuoi — Architecture technique (V1)

> Statut : **brouillon à valider** · Version 0.2 · 2026-11-06
> Changements v0.2 : architecture **hexagonale** explicite, principes SOLID / Clean Code, exigence de **code pédagogique** (mainteneur senior TS/Vue, débutant Rust).

## 0. Exigences de conception (non négociables)

1. **Code pédagogique** : un développeur senior TypeScript/Vue, débutant en Rust, doit pouvoir lire, comprendre et reprendre chaque fichier. Voir §9 et le guide [`03-guide-rust-pour-dev-ts.md`](03-guide-rust-pour-dev-ts.md).
2. **Architecture hexagonale** (ports & adaptateurs) / Clean Architecture : le métier ne dépend de rien de technique.
3. **SOLID** et **Clean Code** : petites unités, noms explicites, une responsabilité par type, dépendances vers des abstractions.
4. **Testabilité** : tout le métier et tous les cas d'usage testables sans disque, sans réseau, sans Tauri.

## 1. Vue d'ensemble

```
                         ┌──────────────── Adaptateurs PRIMAIRES (pilotants) ────────────────┐
                         │  Tauri commands (IPC depuis l'UI Vue)   Watcher du dossier drive   │
                         └───────────────────────────────┬───────────────────────────────────┘
                                                         │ appellent
                     ┌───────────────────────────────────▼───────────────────────────────────┐
                     │                     APPLICATION  (cas d'usage)                         │
                     │  CreateTask · CloseTask · ReopenTask · MoveTask · DeleteTask · Restore │
                     │  EditNote · AttachMedia · SearchTasks · EncryptTask · ApplyExternal…   │
                     │                                                                        │
                     │  PORTS (traits) : TaskRepository · MediaStore · SearchIndex · Cipher   │
                     │  KeyVault · Clock · IdGenerator · FileHydrator · ImageOptimizer ·      │
                     │  AudioRecorder · EventPublisher                                        │
                     │            ┌────────────────────────────────────────────┐              │
                     │            │              DOMAINE (cœur pur)             │              │
                     │            │  Task · TaskId · Tag · Note · Block ·       │              │
                     │            │  TaskTree · ClosurePolicy · ConflictPolicy  │              │
                     │            │  aucune E/S, aucune dépendance technique    │              │
                     │            └────────────────────────────────────────────┘              │
                     └───────────────────────────────────▲───────────────────────────────────┘
                                                         │ implémentent les ports
       ┌─────────────────────────── Adaptateurs SECONDAIRES (pilotés) ──────────────────────────┐
       │ fs_markdown (drive)  sqlite_index  crypto + keyring  cloud_sync (OS)  media  system      │
       └─────────────────────────────────────────────────────────────────────────────────────────┘
```

**Règle de dépendance** : les flèches de dépendance pointent **toujours vers le centre**.
`domaine` ← `application` ← `adaptateurs` ← `composition root` (le binaire Tauri qui assemble tout).
Cette règle est **imposée par le compilateur** : chaque couche est une *crate* Cargo distincte, et la crate `domain` n'a tout simplement pas `rusqlite` ou `tauri` dans ses dépendances — impossible de les importer par erreur.

**Principe de données** : le **dossier du drive est la source de vérité** ; le cache SQLite local est jetable et reconstructible. Aucun serveur, aucun OAuth en V1.

### 1.1 Développement vs exécution — Node.js n'est **jamais** embarqué

| | Développement (`cargo tauri dev`) | Build (`cargo tauri build`) | Exécution (app installée) |
|---|---|---|---|
| Node.js | **Oui** : Vite dev server sur `localhost:5173` (HMR) | **Oui** : `vite build` → fichiers statiques `ui/dist/` | **Non** |
| Rust | Compilé en debug, la webview pointe sur `devUrl` (Vite) | Compilé en release, `ui/dist/` **embarqué dans le binaire** | Le binaire seul |
| Service de la SPA | Vite (HTTP local) | — | **L'app elle-même**, via un protocole interne de la webview (`tauri://localhost` sur macOS/Linux, `http://tauri.localhost` intercepté sous Windows). **Aucun serveur HTTP, aucun port ouvert.** |
| Webview | Celle du système : WebView2 (Windows), WKWebView (macOS), WebKitGTK (Linux) | idem | idem — pas de Chromium embarqué (≠ Electron) |

En exécution : un **processus Rust natif** ouvre des fenêtres système contenant une webview ; la SPA Vue y est chargée depuis les ressources embarquées et dialogue avec le cœur Rust par **IPC** (`invoke` / événements), jamais par HTTP. Les médias passent par le protocole `tq-media://` servi par le même processus.

Bonus de l'hexagonal côté front : l'UI peut aussi tourner **seule dans un navigateur** (`pnpm dev`) avec une passerelle en mémoire (§8), pratique pour travailler l'interface sans compiler Rust.

## 2. Les couches en détail

### 2.1 Domaine — `crates/domain` (`tq_domain`)
Le **vocabulaire métier** et les **règles** qui ne dépendent que d'elles-mêmes.

| Élément | Rôle |
|---|---|
| `TaskId`, `Tag`, `DeviceId` | *Newtypes* (types-valeur) : impossible de passer un `Tag` là où un `TaskId` est attendu, et la validation est faite **une fois**, à la construction (`Tag::parse("client-x")?`). |
| `Task` | Entité : titre, statut, tags, note, parent, ordre, dates, chiffrement. Méthodes métier **intention-révélatrices** : `close(at)`, `reopen(at)`, `rename(..)`, `add_tag(..)` — pas de *setters* génériques, pas de booléen « toggle ». |
| `TaskStatus` | `enum { Open, Done { at } }` — l'état « terminé sans date » est **irreprésentable**. |
| `Note`, `Block` | `Note { blocks: Vec<Block> }` ; `enum Block { Paragraph, Heading, Checklist, RadioGroup, Table, Code, Image, Audio, Attachment, … }`. Le domaine ignore le Markdown. |
| `TaskTree` | Vue en arbre d'un ensemble de tâches : enfants, ancêtres, descendants, détection de cycles (R11). |
| `ClosurePolicy` | Règles R1–R4 : « quels ancêtres proposer à la clôture ? », « quels ancêtres proposer à la réouverture ? ». **Fonctions pures** : entrée = arbre + action, sortie = proposition. |
| `ConflictPolicy` | « Dernier qui écrit gagne » : départage deux versions d'une même tâche (horodatage hybride, puis `DeviceId` pour l'égalité stricte). |
| `OrderKey` | Indice fractionnaire pour l'ordre des frères (`between(a, b)`). |
| `DomainError` | Erreurs métier typées (`EmptyTitle`, `InvalidTag`, `CycleDetected`, …). |

Dépendances autorisées : `thiserror` (dérive des erreurs), `serde` (sérialisation des types, sans format). **Rien d'autre.**

### 2.2 Application — `crates/application` (`tq_application`)
Les **cas d'usage** (un par fichier) et les **ports** (traits) dont ils ont besoin.

```rust
// crates/application/src/use_cases/close_task.rs  (esquisse)

/// Cas d'usage : clôturer une tâche.
/// Renvoie la proposition éventuelle de clôturer aussi les ancêtres (règle R1),
/// que l'UI affichera à l'utilisateur.
pub struct CloseTask {
    tasks: Arc<dyn TaskRepository>,   // port : où sont les tâches ?
    clock: Arc<dyn Clock>,            // port : quelle heure est-il ? (testable)
    events: Arc<dyn EventPublisher>,  // port : prévenir l'UI
}

impl CloseTask {
    pub fn execute(&self, command: CloseTaskCommand) -> Result<CloseTaskOutcome, AppError> {
        let mut tree = self.tasks.load_tree()?;
        let now = self.clock.now();

        let task = tree.get_mut(&command.task_id)?;
        task.close(now)?;
        self.tasks.save(task)?;

        let proposal = ClosurePolicy::ancestors_to_propose(&tree, &command.task_id);
        self.events.publish(DomainEvent::TaskChanged(command.task_id));
        Ok(CloseTaskOutcome { proposal })
    }
}
```

**Ports secondaires** (`crates/application/src/ports/`), volontairement **petits** (ISP) :

| Port | Responsabilité unique |
|---|---|
| `TaskRepository` | Charger / enregistrer / supprimer des tâches (sans savoir que c'est du Markdown). |
| `MediaStore` | Stocker / lire un média par son empreinte. |
| `SearchIndex` | Indexer une tâche, rechercher. |
| `Cipher` | Chiffrer / déchiffrer des octets avec une clé donnée. |
| `KeyVault` | Mémoriser / retrouver la clé du coffre (trousseau OS). |
| `Clock` | Donner l'heure (remplaçable dans les tests). |
| `IdGenerator` | Générer des `TaskId` (ULID). |
| `FileHydrator` | Forcer le téléchargement d'un fichier « à la demande » du drive. |
| `ImageOptimizer` | Réduire / convertir une image. |
| `AudioRecorder` | Démarrer / arrêter une capture micro. |
| `EventPublisher` | Notifier l'extérieur (l'UI) d'un changement. |

**Choix pédagogique — code synchrone** : domaine et application sont **synchrones** (pas d'`async`). Les E/S fichier sont rapides et l'asynchrone Rust est un sujet difficile ; la couche Tauri exécute les cas d'usage dans un thread de travail (`spawn_blocking`) pour ne pas bloquer l'UI. → ADR-0002.

**Choix pédagogique — `Arc<dyn Trait>`** plutôt que des génériques (`CloseTask<R: TaskRepository, C: Clock, …>`) : signatures lisibles, proches de l'injection de dépendances TS (`constructor(private repo: TaskRepository)`). Coût d'exécution négligeable ici. → ADR-0003.

### 2.3 Adaptateurs secondaires — `crates/adapters/*`
Chaque adaptateur **implémente** un ou plusieurs ports, et **traduit** ses erreurs techniques en erreurs du port.

| Crate | Implémente | Technologie |
|---|---|---|
| `fs_markdown` | `TaskRepository`, `MediaStore` | Fichiers du drive, Markdown + front-matter, migrations de schéma, écriture atomique |
| `sqlite_index` | `SearchIndex` (+ état d'UI local) | `rusqlite`, FTS5 |
| `crypto` | `Cipher` | Argon2id, XChaCha20-Poly1305 |
| `keyring` | `KeyVault` | crate `keyring` (Trousseau / Gestionnaire d'identification) |
| `cloud_sync` | `FileHydrator` | Windows Cloud Files API / macOS `NSFileCoordinator`, no-op Linux |
| `media` | `ImageOptimizer`, `AudioRecorder` | `image` + `webp`, `cpal` + Opus/Ogg |
| `system` | `Clock`, `IdGenerator` | horloge système + HLC, `ulid` |

### 2.4 Adaptateurs primaires — `src-tauri`
- **Commandes Tauri** : une fonction fine par action UI, qui convertit le DTO reçu en commande applicative, appelle le cas d'usage, convertit le résultat en DTO. **Aucune règle métier ici.**
- **Watcher du dossier drive** : convertit les événements fichiers en appels au cas d'usage `ApplyExternalChange`.
- **Composition root** (`src-tauri/src/composition.rs`) : le **seul** endroit qui connaît toutes les implémentations concrètes et les assemble (`Arc::new(FsMarkdownTaskRepository::new(..))` → injecté dans `CloseTask::new(..)`). C'est l'équivalent d'un conteneur d'injection, écrit à la main pour rester lisible.

### 2.5 Kit de test — `crates/testing` (`tq_testing`, *dev-dependency* uniquement)
- **Fakes en mémoire** de chaque port (`InMemoryTaskRepository`, `FixedClock`, `SequentialIdGenerator`, `RecordingEventPublisher`…) pour tester les cas d'usage en millisecondes.
- **Tests de contrat** : une même suite de tests (« un `TaskRepository` doit… ») exécutée contre le fake **et** contre l'adaptateur réel → garantit que le fake ne ment pas (principe de substitution de Liskov).
- Constructeurs de données de test (`TaskBuilder::new().titled("…").closed().build()`).

## 3. SOLID et Clean Code appliqués

| Principe | Application concrète dans TodoQuoi |
|---|---|
| **S** — responsabilité unique | Un cas d'usage par fichier ; un port = une responsabilité ; le parsing Markdown (`fs_markdown::format`) séparé de l'accès disque (`fs_markdown::storage`). |
| **O** — ouvert/fermé | Ajouter un drive par API (V2) = **nouvel adaptateur** de `TaskRepository`, zéro modification du domaine ni des cas d'usage. Nouveau type de bloc = nouvelle variante de `Block` + son rendu, le reste du code est guidé par le compilateur (`match` exhaustif). |
| **L** — substitution de Liskov | Tests de contrat partagés entre fake et implémentation réelle. |
| **I** — ségrégation des interfaces | Ports petits et ciblés (cf. §2.2) ; un cas d'usage ne reçoit que les ports dont il se sert. |
| **D** — inversion des dépendances | Les cas d'usage dépendent de traits ; seule la *composition root* connaît les types concrets. |

Règles Clean Code (vérifiées en revue et en partie par `clippy`) :
- Fonctions courtes (cible < 25 lignes), un niveau d'abstraction par fonction, retours anticipés plutôt qu'imbrication.
- Noms révélant l'intention ; pas d'abréviations (sauf `id`) ; pas de paramètre booléen (`close()` / `reopen()` plutôt que `set_done(bool)`).
- Types plutôt que commentaires : newtypes, enums qui rendent les états invalides irreprésentables.
- Pas de `unwrap()` / `expect()` hors tests et hors invariant documenté.
- Pas de duplication de règle métier entre Rust et TS : le front **affiche**, le Rust **décide**.

## 4. Stockage sur le drive (adaptateur `fs_markdown`)

```
TodoQuoi/
├── manifest.json            # schéma, id d'espace, paramètres du coffre, vue racine
├── tasks/
│   ├── 01JC8Z…Q4.md          # tâche en clair
│   └── 01JC90…7B.tqe         # tâche chiffrée
├── media/
│   ├── 3f9a…c2.webp          # adressé par SHA-256 du contenu (immuable)
│   ├── 81bd…0e.ogg
│   └── 5c11…aa.tqe           # média d'une tâche chiffrée
├── .devices/
│   └── mac-greg-7f3a.json    # dernier passage, version app, version schéma
└── .conflicts/
    └── 01JC8Z…Q4.2026-11-05T17-25-12Z.mac-greg-7f3a.md
```

- **Arborescence plate** : la hiérarchie est portée par `parent` ; un déplacement modifie **un** champ dans **un** fichier (les renommages de fichiers sont le point faible des clients de synchro).
- **Ordre par indice fractionnaire** (`order`) : réordonner ne touche qu'un fichier.
- **Médias immuables adressés par hash** : jamais de conflit, déduplication gratuite ; liens relatifs (`../media/…`) → note lisible dans un autre éditeur Markdown.
- **Corbeille par champ** `deleted_at`.

### 4.1 Format d'une tâche

```markdown
---
schema_version: 1
id: 01JC8ZK3V5X9G2M7T4QW1R6YQ4
parent: 01JC8Z9A1B2C3D4E5F6G7H8J9K
order: "a0V"
title: Préparer la release 1.2
done: false
done_at: null
tags: [release, client-x]
created_at: 2026-11-05T16:02:11.120Z
updated_at: 2026-11-05T17:25:12.345Z
updated_by: mac-greg-7f3a
deleted_at: null
encrypted: false
view:
  columns: [title, progress, tags, updated_at]
  sort: [{ field: manual }]
  filter: { status: open, tags_in: [], tags_out: [] }
---
Checklist de la release :

- [x] Changelog
- [ ] Tag Git

:::radio{label="Cible"}
- ( ) Recette
- (x) Production
:::

![capture](../media/3f9a…c2.webp){width=480}

::audio{src="../media/81bd…0e.ogg" duration=12.4}
```

- `fs_markdown::format` : `Note` (domaine) ↔ Markdown étendu (`pulldown-cmark` + directives `:::radio`, `::audio`, `::file`). **Test de propriété** : `parse(render(note)) == note`.
- Le front manipule un JSON de blocs (DTO) converti en `Note` par la couche Tauri : **une seule conversion vers le Markdown, en Rust, testée**.
- Champs inconnus du front-matter **préservés** (compatibilité avant).
- `fs_markdown::migrations` : une fonction par saut de version (`v1_to_v2`), idempotente, testée sur des fichiers d'exemple.

### 4.2 Tâche chiffrée (`.tqe`)
```
"TQE1" | u32 len | en-tête JSON en clair | nonce 24 o | texte chiffré (+ tag Poly1305)
```
- En-tête en clair minimal (arbre + synchro) : `schema_version, id, parent, order, done, updated_at, updated_by, deleted_at, key_id`.
- Chiffré : le `.md` complet. AAD = en-tête → toute altération invalide le déchiffrement.
- Médias de la tâche : même format ; nom = HMAC(clé, contenu) pour ne rien révéler.

## 5. Synchronisation

### 5.1 Écriture (adaptateur `fs_markdown::storage`)
1. Sérialiser → écrire `tasks/.~<id>.tmp` → `fsync` → `rename` atomique.
2. `updated_at` = horloge hybride (adaptateur `system::HybridClock`) : monotone même si les horloges des appareils dérivent.
3. Débounce 1 s par tâche (couche Tauri) ; flush à la fermeture.

### 5.2 Changements externes (adaptateur primaire « watcher »)
- `notify` (ReadDirectoryChangesW / FSEvents / inotify) + **scan de réconciliation** périodique (60 s) et au réveil.
- Fichier changé → cas d'usage `ApplyExternalChange` → `ConflictPolicy` (domaine) départage → index mis à jour → événement UI.

### 5.3 Conflits (« dernier qui écrit gagne »)
| Situation | Traitement |
|---|---|
| Fichier modifié sur disque alors qu'on a des modifs locales non écrites | `ConflictPolicy` : le plus récent gagne ; le perdant → `.conflicts/`. Tâche ouverte : bandeau Recharger / Garder la mienne. |
| Copie de conflit du drive (`x (conflicted copy).md`, `x-DESKTOP-ABC.md`, `x (1).md`…) | Détection par motifs + `id` identique : idem. |
| Fichier illisible / front-matter invalide | Ignoré, signalé ⚠, jamais réécrit. |

### 5.4 Aides OS (adaptateur `cloud_sync`, port `FileHydrator`)
| OS | Problème | Solution |
|---|---|---|
| Windows (OneDrive, GDrive, Dropbox via Cloud Files API) | Fichiers « à la demande » | Détecter `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` / `RECALL_ON_OPEN` ; `CfHydratePlaceholder` ; proposer d'**épingler** le dossier. Crate `windows`. |
| macOS iCloud | Fichiers évincés | `NSURLUbiquitousItemDownloadingStatusKey`, `startDownloadingUbiquitousItem`. Crate `objc2-foundation`. |
| macOS File Provider (`~/Library/CloudStorage`) | Fichiers non matérialisés | Lecture coordonnée (`NSFileCoordinator`). |
| Linux | — | Implémentation no-op. |

Le code spécifique à chaque OS est isolé dans un module par OS (`#[cfg(target_os = "…")]`), derrière le même trait : le reste du programme ignore sur quel OS il tourne.

## 6. Cache local (adaptateur `sqlite_index`)
Emplacement (crate `directories`) : `%LOCALAPPDATA%\TodoQuoi\` · `~/Library/Application Support/TodoQuoi/` · `$XDG_DATA_HOME/TodoQuoi/`.

`cache.db` (SQLite WAL) : table `tasks` (projection pour l'affichage), `tasks_fts` (FTS5, `unicode61 remove_diacritics 2`), `ui_state` (dépliage, fenêtres — local à l'appareil). Tâches chiffrées : index FTS **en mémoire** uniquement, détruit au verrouillage. Cache perdu / corrompu / schéma changé → reconstruction depuis le drive.

## 7. Crypto et médias
- **Crypto** (`crypto`, `keyring`) : Argon2id (m = 64 Mio, t = 3, p = 1), sel dans `manifest.json` ; clé maître 256 bits + valeur de contrôle ; XChaCha20-Poly1305, nonce aléatoire par écriture ; clé en mémoire dans `zeroize::Zeroizing` ; clé de contenu enveloppée (changement de phrase de passe sans tout rechiffrer).
- **Images** (`media`) : `image` + `webp`, orientation EXIF appliquée puis métadonnées supprimées.
- **Audio** (`media`) : capture côté Rust (`cpal`) → Opus → Ogg, format identique sur tous les OS. **Risque** : lecture Ogg/Opus par WKWebView → POC I0 ; repli WAV via `tq-media://`.
- Tous les médias servis au front par le protocole `tq-media://<hash>` (déchiffrement à la volée), jamais d'accès direct du front au disque.

## 8. Front (Vue 3) — même découpage

```
ui/src/
├── domain/          # types TS générés depuis Rust (tauri-specta) : DTO, jamais réécrits à la main
├── application/     # stores Pinia / composables : orchestration côté UI (sélection, fenêtres, filtres)
├── ports/           # interface TaskGateway, MediaGateway… (ce que l'UI attend du backend)
├── infrastructure/
│   ├── tauri/       # TauriTaskGateway : implémente les ports via invoke()/listen()
│   └── in-memory/   # InMemoryTaskGateway : données fictives, pour Vitest et `pnpm dev` dans un navigateur
└── presentation/    # composants Vue (arbre, panneau, éditeur TipTap et ses extensions)
```
Les composants ne font **jamais** `invoke()` directement : ils passent par un port, injecté via `provide/inject`.

| Besoin | Choix |
|---|---|
| Build | Vite + TypeScript strict |
| Arbre virtualisé | `@tanstack/vue-virtual` + composant arbre maison (ARIA `tree`) |
| Éditeur | TipTap 2 (`@tiptap/vue-3`) + extensions `RadioGroup`, `AudioBlock`, `FileBlock`, `TaskLink`, `ImageResizable` |
| i18n | `fluent-vue` |
| Fenêtres de tâche | `WebviewWindow` Tauri, route `/task/:id` |

## 9. Code pédagogique — conventions

Objectif : un lecteur qui connaît TypeScript mais pas Rust comprend **chaque fichier** sans aide extérieure.

1. **En-tête de module** (`//!`) dans chaque fichier : rôle du module, **sa place dans l'hexagone** (domaine / application / adaptateur), ce qu'il ne doit pas faire.
2. **Doc publique** (`///`) sur tout élément public : quoi, pourquoi, erreurs possibles, exemple exécutable (*doctest*) quand c'est utile.
3. **Notes Rust balisées** `// [rust]` à la **première occurrence** d'un concept dans un fichier, reliées à l'équivalent TS. Exemple :
   ```rust
   // [rust] `?` = « si c'est une erreur, la renvoyer immédiatement à l'appelant ».
   //        Équivalent TS : un `throw` implicite, mais visible dans le type de retour `Result<T, E>`.
   let task = tree.get_mut(&id)?;
   ```
   Recherche rapide de toutes les notes : `rg "\[rust\]"`.
4. **Sous-ensemble de Rust volontairement simple** :
   - types possédés (`String`, `Vec<T>`) dans les structures ; références `&` en paramètre ; **pas de durées de vie explicites** (`'a`) sauf nécessité documentée ;
   - pas de macros maison, pas de *trait objects* imbriqués, pas de génériques au-delà du nécessaire ;
   - `Arc<dyn Port>` pour l'injection (§2.2) ; `Mutex` seulement dans les adaptateurs, jamais dans le domaine ;
   - erreurs : `thiserror` dans les bibliothèques, `anyhow` uniquement dans `src-tauri` ;
   - pas d'`unsafe` hors `cloud_sync` (appels système), chaque bloc justifié par un commentaire `// SAFETY:`.
5. **Identifiants en anglais**, **commentaires et documentation en français**.
6. **ADR** (`docs/adr/NNNN-titre.md`) pour toute décision structurante, au format Contexte / Décision / Conséquences.
7. **Outillage** : `rustfmt` (formatage), `clippy` en CI avec `-D warnings` (+ lints `clippy::unwrap_used`, `clippy::too_many_lines`), `cargo doc` publié localement (`cargo doc --open`) — la doc générée est un vrai support de lecture.

## 10. Organisation du dépôt

```
TodoQuoi/
├── Cargo.toml                       # workspace
├── crates/
│   ├── domain/                      # tq_domain        — cœur pur
│   ├── application/                 # tq_application   — cas d'usage + ports
│   ├── adapters/
│   │   ├── fs_markdown/             # TaskRepository, MediaStore
│   │   ├── sqlite_index/            # SearchIndex
│   │   ├── crypto/                  # Cipher
│   │   ├── keyring/                 # KeyVault
│   │   ├── cloud_sync/              # FileHydrator (par OS)
│   │   ├── media/                   # ImageOptimizer, AudioRecorder
│   │   └── system/                  # Clock, IdGenerator
│   └── testing/                     # fakes + tests de contrat (dev-dependency)
├── src-tauri/                       # adaptateurs primaires + composition root
│   └── src/
│       ├── main.rs
│       ├── composition.rs           # assemblage des dépendances
│       ├── commands/                # une commande Tauri par fichier
│       ├── dto/                     # objets échangés avec l'UI
│       └── watcher.rs
├── ui/                              # Vue 3 + Vite (cf. §8)
├── docs/
│   ├── 01-specification-fonctionnelle.md
│   ├── 02-architecture.md
│   ├── 03-guide-rust-pour-dev-ts.md
│   └── adr/
├── maquette/index.html
├── CLAUDE.md                        # conventions du projet (pour les assistants IA et les humains)
├── .gitattributes                   # * text=auto eol=lf
└── .editorconfig
```

Portabilité : chemins via `std::path` / `directories` uniquement ; fins de ligne **LF** forcées ; noms de fichiers = ULID (ASCII, insensibles à la casse).

## 11. Stratégie de test

| Couche | Type de test | Outils |
|---|---|---|
| Domaine | Unitaires + propriétés (règles R1–R12, arbre, ordre, conflits) | `cargo test`, `proptest` |
| Application | Cas d'usage avec fakes en mémoire (rapides, sans disque) | `tq_testing` |
| Adaptateurs | Tests de contrat + intégration (dossier temporaire, deux « appareils » concurrents, copies de conflit simulées) | `tempfile` |
| `fs_markdown::format` | Aller-retour Markdown sur corpus, migrations | fichiers d'exemple |
| Front | Composants et stores avec `InMemoryTaskGateway` | Vitest |
| Bout en bout | Quelques parcours clavier | Playwright + `tauri-driver` |

CI : GitHub Actions, matrice `windows-latest` / `macos-latest` : `fmt --check`, `clippy -D warnings`, `test`, `doc`.

## 12. Plan par incréments

| # | Contenu | Livrable vérifiable |
|---|---|---|
| **I0** | **POC des risques** : aller-retour TipTap ↔ Markdown étendu ; capture + lecture Ogg/Opus sur WebView2 et WKWebView ; hydratation OneDrive Windows + iCloud macOS | 3 mini-démos, go/no-go |
| I1 | Workspace, `domain` (Task, arbre, règles de clôture), `application` (CRUD), `testing`, `fs_markdown` (sans watcher), squelette Tauri + Vue, arbre, clavier | Gérer des tâches dans un dossier local |
| I2 | Panneau + fenêtres de tâche, éditeur (texte, liens, cases, tableaux, code), tags | Notes riches sans médias |
| I3 | `media` : images, pièces jointes, audio ; groupes radio | Notes complètes |
| I4 | `sqlite_index` : recherche, vues par liste + héritage, zoom | Recherche et présentation |
| I5 | Synchro : watcher, réconciliation, conflits, `cloud_sync`, indicateur, corbeille | Usage réel sur 2 appareils |
| I6 | `crypto` + `keyring` : coffre, tâches chiffrées | Chiffrement par tâche |
| I7 | Finitions : i18n, thèmes, accessibilité, installeurs MSI / DMG | V1 |

Chaque incrément se termine par une **revue de lisibilité** : un passage « est-ce qu'un débutant Rust comprend ce fichier ? ».

## 13. Risques

| Risque | Impact | Parade |
|---|---|---|
| Aller-retour Markdown ↔ éditeur imparfait | Perte de mise en forme | Conversion unique en Rust, tests de propriété, champs inconnus préservés |
| Watcher incomplet sur dossiers File Provider | Changements non vus | Scan de réconciliation |
| « Dernier qui écrit gagne » écrase une modif | Perte de données | `.conflicts/` + journal, HLC, débounce court |
| Ogg/Opus dans WKWebView | Audio illisible sur Mac | POC I0, repli WAV |
| Perte de la phrase de passe | Données chiffrées perdues | Avertissement, clé de secours exportable |
| Courbe d'apprentissage Rust | Lenteur, code difficile à reprendre | Sous-ensemble simple (§9), notes `[rust]`, guide `03`, revues de lisibilité |
| Sur-ingénierie hexagonale pour une petite app | Verbosité | Ports limités au strict nécessaire ; pas de port « au cas où » |

## 14. Dépendances à installer (à valider — **Modification Système**)
Rien n'est installé à ce stade. Pour I0/I1 : Rust stable (`rustup`), Node.js LTS + pnpm, Tauri CLI ; macOS : Xcode Command Line Tools ; Windows : Build Tools MSVC (WebView2 présent sur Windows 11). Commandes proposées au moment voulu, pour accord.

## 15. ADR à rédiger en I1
- ADR-0001 — Architecture hexagonale en crates Cargo séparées.
- ADR-0002 — Domaine et application synchrones.
- ADR-0003 — Injection par `Arc<dyn Trait>` plutôt que génériques.
- ADR-0004 — Un fichier Markdown par tâche, « dernier qui écrit gagne ».
- ADR-0005 — Tauri 2 + Vue 3.
