# Décisions d'architecture (ADR)

Une ADR (*Architecture Decision Record*) consigne **une** décision structurante : son contexte,
la décision, et ses conséquences. On n'en modifie pas le fond après coup : si la décision change,
on écrit une nouvelle ADR qui remplace l'ancienne (statut « Remplacée par ADR-XXXX »).

| N° | Titre | Statut |
|---|---|---|
| [0001](0001-architecture-hexagonale.md) | Architecture hexagonale en crates séparées | Acceptée |
| [0002](0002-domaine-synchrone.md) | Domaine et application synchrones | Acceptée |
| [0003](0003-injection-arc-dyn.md) | Injection des dépendances par `Arc<dyn Trait>` | Acceptée |
| [0004](0004-stockage-markdown-toml.md) | Un fichier Markdown + TOML par tâche, « dernier qui écrit gagne » | Acceptée |
| [0005](0005-tauri-vue.md) | Tauri 2 + Vue 3 | Acceptée |
| [0006](0006-politique-dependances.md) | Politique de dépendances | Acceptée |

Modèle : copier une ADR existante, garder les sections *Contexte / Décision / Conséquences*.
