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

## 9. Commandes utiles

| Commande | Effet |
|---|---|
| `cargo build` | Compiler |
| `cargo test` | Tous les tests du workspace |
| `cargo test -p tq_domain` | Tests d'une crate |
| `cargo clippy --all-targets -- -D warnings` | Lint strict |
| `cargo fmt` | Formater |
| `cargo doc --open` | Documentation générée, navigable |
| `cargo tauri dev` | Lancer l'app en développement |

## 10. Lectures recommandées
- *The Rust Programming Language* (« le Book »), chapitres 4 (ownership), 6 (enums), 9 (erreurs), 10 (traits), 17 (trait objects).
- *Rust by Example* pour les exemples courts.
- Documentation des crates : `docs.rs/<nom_de_crate>`.
