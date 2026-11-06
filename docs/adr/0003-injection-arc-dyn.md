# ADR-0003 — Injection des dépendances par `Arc<dyn Trait>`

- Statut : Acceptée · Date : 2026-11-06

## Contexte
Un cas d'usage dépend de ports (dépôt de tâches, horloge…). Rust offre deux façons de le dire :
- des **génériques** : `CloseTask<R: TaskRepository, C: Clock>` — résolus à la compilation ;
- des **objets trait** : `CloseTask { tasks: Arc<dyn TaskRepository>, … }` — résolus à l'exécution.

## Décision
Objets trait partagés : `Arc<dyn Port>` (un `Arc` est un pointeur partagé, compté et utilisable
depuis plusieurs fils). Les ports sont déclarés `: Send + Sync`.

## Conséquences
- ➕ Signatures courtes et lisibles, proches de l'injection par constructeur en TypeScript
  (`constructor(private readonly tasks: TaskRepository)`).
- ➕ La composition root (`src-tauri`) assemble tout explicitement, sans conteneur magique.
- ➖ Un appel indirect par méthode : coût négligeable pour cette application.
