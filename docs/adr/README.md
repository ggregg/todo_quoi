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
| [0007](0007-note-markdown-a-la-frontiere.md) | La note reste du Markdown ; conversion à la frontière | Acceptée |
| [0008](0008-synchro-sans-dependance.md) | Synchronisation par le dossier du drive sans dépendance système | Acceptée |
| [0009](0009-plusieurs-bases.md) | Plusieurs bases de tâches ouvertes ensemble | Acceptée |
| [0010](0010-un-coffre-par-base.md) | Un coffre par base, créé par un mot de passe à la première utilisation | Acceptée |
| [0011](0011-fichiers-lies.md) | Joindre, déplacer ou lier un fichier ; liens propres à un appareil | Acceptée |
| [0012](0012-alignement-et-couleur-du-texte.md) | Alignement et couleur du texte des notes | Acceptée |

Modèle : copier une ADR existante, garder les sections *Contexte / Décision / Conséquences*.
