# Guide Rust pour développeur TypeScript — ce qu'il faut savoir pour lire TodoQuoi

> Public : développeur senior TS/Vue, débutant Rust.
> Portée : **uniquement** les notions utilisées dans ce dépôt (cf. sous-ensemble défini dans `02-architecture.md` §9).
> Dans le code, chaque première utilisation d'une notion est signalée par un commentaire `// [rust]` — `rg "\[rust\]"` pour les retrouver.

## 1. Correspondances rapides

| TypeScript | Rust | Remarque |
|---|---|---|
| `package.json` / workspace pnpm | `Cargo.toml` / workspace Cargo | Une *crate* ≈ un package. |
| `import { X } from './x'` | `use crate::x::X;` | `crate::` = racine de la crate courante. |
| `export` | `pub` | Tout est **privé par défaut**. |
| `interface Repo { save(t: Task): void }` | `trait TaskRepository { fn save(&self, t: &Task) -> Result<(), RepoError>; }` | Un trait ≈ une interface. |
| `class FsRepo implements Repo` | `struct FsRepo { … }` + `impl TaskRepository for FsRepo { … }` | Données (`struct`) et comportement (`impl`) séparés. |
| `constructor(private repo: Repo)` | `fn new(repo: Arc<dyn TaskRepository>) -> Self` | Pas de constructeur spécial : `new` est une convention. |
| `type Status = 'open' \| 'done'` | `enum TaskStatus { Open, Done { at: Timestamp } }` | Les variantes peuvent porter des données. |
| `switch` | `match` | **Exhaustif** : oublier un cas = erreur de compilation. |
| `T \| undefined` | `Option<T>` (`Some(x)` / `None`) | Pas de `null`. |
| `throw` / `try…catch` | `Result<T, E>` + `?` | L'erreur fait partie du type de retour. |
| `readonly` | par défaut | Une variable est **immuable** sauf `let mut`. |
| `string` | `String` (possédée) / `&str` (empruntée) | Cf. §3. |
| `T[]` | `Vec<T>` | |
| `Map<K, V>` | `HashMap<K, V>` / `BTreeMap<K, V>` (triée) | |
| `JSON.stringify` / zod | `serde` (`#[derive(Serialize, Deserialize)]`) | |
| `vitest` | `cargo test` + `#[test]` | Tests unitaires dans le même fichier (`mod tests`). |
| `eslint` / `prettier` | `cargo clippy` / `cargo fmt` | |
| JSDoc | `///` (élément) et `//!` (module) | `cargo doc --open` génère le site. |

## 2. Les erreurs : `Result` et `?`

```rust
// Une fonction qui peut échouer le dit dans sa signature.
fn parse_tag(raw: &str) -> Result<Tag, DomainError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DomainError::InvalidTag(raw.to_string())); // ≈ throw
    }
    Ok(Tag(trimmed.to_lowercase()))                           // ≈ return
}

fn add_tag(task: &mut Task, raw: &str) -> Result<(), DomainError> {
    let tag = parse_tag(raw)?;   // `?` : si Err → on renvoie l'erreur tout de suite, sinon on déballe la valeur
    task.tags.push(tag);
    Ok(())
}
```

- Chaque couche a son type d'erreur (`DomainError`, `AppError`, `RepositoryError`…), déclaré avec `thiserror` (une simple *derive*).
- Passer d'une erreur à l'autre : `impl From<DomainError> for AppError` — alors `?` convertit automatiquement.
- `unwrap()` = « je suis sûr que ce n'est pas une erreur, sinon plante » : **interdit** hors tests dans ce projet.

## 3. Propriété (*ownership*) et emprunt — la notion clé

En TS, le ramasse-miettes gère la mémoire. En Rust, **chaque valeur a un seul propriétaire** ; quand il disparaît, la valeur est libérée. Pour utiliser une valeur sans la prendre, on l'**emprunte** :

| Écriture | Sens | Analogie TS |
|---|---|---|
| `fn f(t: Task)` | f **prend possession** de `t` ; l'appelant ne peut plus l'utiliser | aucune — comme si la variable était « consommée » |
| `fn f(t: &Task)` | f **lit** `t` (emprunt partagé) | passer un objet en lecture seule |
| `fn f(t: &mut Task)` | f **modifie** `t` (emprunt exclusif) | passer un objet mutable, en étant seul à y toucher pendant l'appel |

Règle du compilateur : **plusieurs lecteurs OU un seul écrivain**, jamais les deux à la fois. C'est ce qui rend les *data races* impossibles.

Conventions du projet pour éviter les difficultés :
- les `struct` **possèdent** leurs données (`String`, `Vec`) → pas de durées de vie (`'a`) à écrire ;
- les fonctions **empruntent** leurs paramètres (`&Task`, `&str`) sauf si elles doivent les conserver ;
- en cas de doute, `.clone()` est acceptable : la lisibilité prime sur la micro-optimisation.

`String` vs `&str` : `String` est une chaîne possédée (modifiable, allouée) ; `&str` est une vue en lecture sur une chaîne. Paramètres en `&str`, champs en `String`.

## 4. Traits et injection de dépendances (l'hexagone)

```rust
// Port (couche application) — l'équivalent d'une interface TS.
pub trait Clock: Send + Sync {          // Send + Sync : utilisable depuis plusieurs threads
    fn now(&self) -> Timestamp;
}

// Adaptateur réel (crate adapters/system).
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Timestamp { Timestamp::from_system_time(std::time::SystemTime::now()) }
}

// Fake pour les tests (crate testing).
pub struct FixedClock(pub Timestamp);
impl Clock for FixedClock {
    fn now(&self) -> Timestamp { self.0 }
}

// Cas d'usage : dépend du trait, pas de l'implémentation.
pub struct CloseTask { clock: Arc<dyn Clock>, /* … */ }
```

- `dyn Clock` = « n'importe quel type qui implémente `Clock` », choisi à l'exécution (comme une interface TS).
- `Arc<…>` = pointeur partagé à comptage de références (*Atomic Reference Counted*) : plusieurs cas d'usage peuvent partager le même dépôt. ≈ une référence d'objet en TS.
- La *composition root* (`src-tauri/src/composition.rs`) fait l'équivalent de `new CloseTask(new SystemClock(), …)`.

## 5. `struct`, `impl`, `enum`, `match`

```rust
pub enum TaskStatus {
    Open,
    Done { at: Timestamp },
}

impl Task {
    /// Clôture la tâche. Erreur si elle l'est déjà.
    pub fn close(&mut self, at: Timestamp) -> Result<(), DomainError> {
        match self.status {
            TaskStatus::Done { .. } => Err(DomainError::AlreadyClosed(self.id.clone())),
            TaskStatus::Open => {
                self.status = TaskStatus::Done { at };
                self.updated_at = at;
                Ok(())
            }
        }
    }
}
```

- `&mut self` : la méthode modifie l'objet (≈ méthode non `readonly`). `&self` : lecture seule.
- `..` dans un motif : « ignorer les autres champs ».
- Le `match` est exhaustif : ajouter une variante à `TaskStatus` signale **tous** les endroits à mettre à jour. C'est l'outil n°1 du principe ouvert/fermé côté domaine.

## 6. *Newtypes* : des types pour éviter les erreurs

```rust
pub struct TaskId(String);   // une String « étiquetée » : on ne peut pas confondre TaskId et Tag
pub struct Tag(String);
```
≈ les *branded types* TS (`type TaskId = string & { __brand: 'TaskId' }`), mais vérifiés sans astuce.

## 7. Modules et visibilité

```
crates/domain/src/
├── lib.rs          // racine : déclare les modules (`pub mod task;`)
├── task.rs         // module `task`
└── tree/
    ├── mod.rs      // module `tree`
    └── closure.rs  // sous-module `tree::closure`
```
- `pub` : visible hors du module ; `pub(crate)` : visible dans la crate seulement.
- `mod tests` en bas d'un fichier, annoté `#[cfg(test)]` : compilé uniquement pour `cargo test`.

## 8. Attributs et macros que vous croiserez

| Écriture | Sens |
|---|---|
| `#[derive(Debug, Clone, PartialEq)]` | Génère automatiquement l'affichage de debug, la copie, la comparaison. |
| `#[derive(Serialize, Deserialize)]` | Sérialisation `serde` (≈ JSON automatique). |
| `#[derive(thiserror::Error)]` + `#[error("…")]` | Déclare un type d'erreur et son message. |
| `#[tauri::command]` | Expose une fonction à l'UI (`invoke('close_task', …)`). |
| `#[cfg(target_os = "windows")]` | Code compilé seulement sous Windows. |
| `#[test]` | Fonction de test. |
| `println!`, `format!`, `vec![]` | Macros standard (le `!` signale une macro). |

## 9. Notions rencontrées dans le code (I1 → I6)

Complément des sections précédentes : chaque notion est expliquée **sur place** par une note `// [rust]`
dans le fichier indiqué.

### Concurrence et partage

| Notion | Sens | Équivalent TS | Où |
|---|---|---|---|
| `Send + Sync` | Le type peut être envoyé / partagé entre fils d'exécution | aucun (JS est mono-fil) | bornes des ports, `application/src/ports` |
| `Mutex<T>` | Verrou : un seul fil à la fois modifie `T` ; `lock()` renvoie un garde | aucun | fakes, `testing/src/fakes.rs` |
| `RwLock<T>` | Verrou « plusieurs lecteurs OU un seul écrivain » : les recherches se font en parallèle | aucun | `memory_index/src/lib.rs` |
| Mutex « empoisonné » | Un fil a paniqué en tenant le verrou ; `lock()` renvoie alors une erreur | — | `testing/src/fakes.rs` |
| `AtomicU64::fetch_add` | « Lire puis ajouter 1 » sans verrou, sûr entre fils | — | `SteppingClock`, `SequentialIdGenerator` (`testing/src/fakes.rs`) |
| `async fn` + `spawn_blocking` | La commande Tauri attend un fil de travail sans bloquer l'UI | `await` d'un Worker | `src-tauri/src/commands/mod.rs` (`run`) |
| `'static` (borne) | La valeur ne contient aucune référence temporaire : elle peut partir dans un autre fil | — | `run` |

Pourquoi le domaine et l'application restent **synchrones** : ADR-0002. Seule la coquille Tauri est asynchrone.

### Fonctions en paramètre (closures)

| Écriture | Sens | Équivalent TS |
|---|---|---|
| `\|x\| x + 1` | Closure (fonction anonyme) | `(x) => x + 1` |
| `move \|\| …` | La closure **prend possession** des variables capturées (nécessaire pour l'envoyer à un autre fil) | — |
| `impl Fn(A) -> R` | « N'importe quelle fonction de cette forme », appelable plusieurs fois | `(a: A) => R` |
| `impl FnOnce() -> R` | Appelable **une seule fois** (elle peut consommer ce qu'elle capture) | — |

Exemples : `note_markdown/src/render.rs` (rendu paramétré), `src-tauri/src/composition.rs` (constructeurs
`CloseTask::new`… passés comme des valeurs).

### Itérateurs

| Écriture | Équivalent TS |
|---|---|
| `items.iter().map(f).collect::<Vec<_>>()` | `items.map(f)` (paresseux jusqu'au `collect`) |
| `.filter(p)`, `.any(p)`, `.find(p)` | `.filter`, `.some`, `.find` |
| `.partition(p)` | deux `filter` complémentaires en une passe (`parse.rs`) |
| `.by_ref()` | Continuer d'avancer dans le **même** itérateur après une boucle (`directives.rs`) |
| `impl IntoIterator<Item = T>` en paramètre | Accepte `Vec`, tableau, itérateur… (≈ `Iterable<T>`) (`session.rs`) |

### Traits standard à connaître

| Trait | Effet | Équivalent TS | Où |
|---|---|---|---|
| `Display` | `format!("{id}")`, `id.to_string()` | `toString()` | `domain/src/identifiers.rs` |
| `From` / `#[from]` | Conversion automatique, utilisée par `?` | — | `application/src/error.rs` |
| `Copy` | Copie implicite à l'affectation (petites valeurs) | `number` | `domain/src/timestamp.rs` |
| `Drop` | Code exécuté quand la valeur disparaît (nettoyage) | `finally` / `using` | `fs_markdown/tests/storage.rs` |
| `Zeroizing<T>` (crate `zeroize`) | Enveloppe dont le `Drop` remplit la mémoire de zéros : secrets effacés à la libération | aucun (le ramasse-miettes JS ne le permet pas) | `application/src/vault/secrets.rs` |
| `Debug` écrit à la main | Afficher `Password(…)` au lieu du contenu : un secret ne finit jamais dans un journal | `toJSON()` personnalisé | `secrets.rs` |

### Syntaxe

| Écriture | Sens | Où |
|---|---|---|
| `Struct { champ: v, ..autre }` | Reprendre les autres champs (≈ `{ ...autre, champ: v }`) | `note_markdown/src/pm.rs` |
| `if let Some(x) = a && cond && let Ok(y) = b` | Chaîne de conditions : tout doit réussir (édition 2024) | `note_markdown/src/parse.rs` |
| `match x { Motif if garde => … }` | Garde sur un bras de `match` | `parse.rs` |
| `opt?` sur un `Option` | Renvoie `None` tout de suite si absent | `note_markdown/src/directives.rs` |
| `fn f<const N: usize>() -> [u8; N]` | Paramètre générique qui est une **valeur** connue à la compilation (taille d'un tableau) | `crypto/src/random.rs` |
| `x as Arc<dyn Trait>` | Oublier le type précis pour ne garder que l'interface (≈ typer une variable par son interface) | `keyring/src/lib.rs` |
| `let Some(x) = opt else { return … };` | Déstructurer ou sortir tout de suite (*let-else*, ≈ garde `if (!x) return`) | `fs_markdown/src/storage.rs` |
| `'t` (durée de vie) | « Le résultat emprunte dans ce paramètre et ne peut pas lui survivre » — **exception documentée** au sous-ensemble | `parse.rs`, `domain/src/rules/hierarchy.rs` |

### Attributs

| Écriture | Sens |
|---|---|
| `#[serde(rename_all = "camelCase")]` | Champs en camelCase dans le JSON envoyé à l'UI (`dto.rs`) |
| `#[serde(tag = "kind")]` | Énumération ⇄ union discriminée TS `{ kind: 'a', … }` (`dto.rs`) |
| `#[expect(clippy::…, reason = "…")]` | Exception de lint **justifiée** ; avertit si elle devient inutile |
| `#[cfg(test)]`, `#[cfg(target_os = "macos")]` | Compilation conditionnelle (tests, OS) ; une fonction par OS, même signature (`keyring/src/lib.rs`, `fs_markdown/src/file_state.rs`) |
| `[target.'cfg(target_os = "macos")'.dependencies]` | Dans `Cargo.toml` : dépendance tirée seulement pour cet OS (`crates/adapters/keyring/Cargo.toml`) |
| `[profile.dev.package.argon2] opt-level = 3` | Optimiser une seule dépendance même en développement (`Cargo.toml` racine) |

### Tauri

| Écriture | Sens | Où |
|---|---|---|
| `.manage(state)` | Enregistre l'état partagé au démarrage | `src-tauri/src/lib.rs` |
| `State<'_, AppState>` | Tauri injecte cet état dans la commande | `commands/*.rs` |
| `#[tauri::command]` + `generate_handler![…]` | Expose une fonction à `invoke()` | `commands/`, `lib.rs` |
| `tauri::ipc::Request` | Corps binaire brut (médias), sans passer par JSON | `commands/media.rs` |
| `register_asynchronous_uri_scheme_protocol` | Protocole `tq-media://` servant les médias | `media_protocol.rs` |

## 10. Commandes utiles

| Commande | Effet |
|---|---|
| `cargo build` | Compiler |
| `cargo test` | Tous les tests du workspace |
| `cargo test -p tq_domain` | Tests d'une crate |
| `cargo clippy --all-targets -- -D warnings` | Lint strict |
| `cargo fmt` | Formater |
| `cargo doc --open` | Documentation générée, navigable |
| `pnpm dev` | Lancer l'app en développement (le CLI Tauri est une dépendance de dev du dépôt) |

Toutes les commandes du projet : [`04-guide-developpeur.md`](04-guide-developpeur.md).

## 11. Lectures recommandées
- *The Rust Programming Language* (« le Book »), chapitres 4 (ownership), 6 (enums), 9 (erreurs), 10 (traits), 17 (trait objects).
- *Rust by Example* pour les exemples courts.
- Documentation des crates : `docs.rs/<nom_de_crate>`.
