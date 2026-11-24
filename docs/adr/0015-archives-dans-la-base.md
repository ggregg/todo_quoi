# ADR-0015 — Archives : un dossier `archive/` dans chaque base

- Statut : Acceptée · Date : 2026-11-24 · Complète l'ADR-0004 (un fichier par tâche) et R5–R8

## Contexte
Le mainteneur veut **archiver** des tâches à la main : elles quittent les listes sans être
supprimées, restent consultables sur demande et peuvent être restaurées. Il veut aussi qu'elles
soient **rangées à part sur le disque**. La corbeille, elle, n'est qu'un champ (`deleted_at`) dans
`tasks/`. Contraintes : synchronisation par le dossier du drive (ADR-0008), coffre par base
(ADR-0010), médias partagés et purgés quand plus rien ne les cite (R8).

## Décision
- **Où** : `<base>/archive/<id>.md`, à côté de `tasks/`. Même format de fichier, plus le champ
  `archived_at`. Même base, donc même synchronisation, même coffre (une note chiffrée reste
  chiffrée, sans déchiffrement pour archiver), médias inchangés dans `media/`.
- **Quoi** : la tâche et ses sous-tâches actives, ensemble (même `archived_at`, R13) ; n'importe
  quelle tâche, avec confirmation si le sous-arbre contient des tâches ouvertes.
- **Lecture seule** (R14) : une archive ne s'écrit qu'en entrant (archiver) ou en sortant
  (restaurer, mettre à la corbeille). Moins d'écritures concurrentes dans `archive/`.
- **Affichage à leur place** (R15), à la demande ; hors de cette option, `archive/` n'est pas relu
  toutes les 3 s : seulement pour l'afficher, restaurer, ou calculer les médias encore cités.
- **Règles dans le domaine** (`tq_domain::rules::archive`, comme `trash.rs`) : sous-arbre à
  archiver et son impact (R13), place de restauration (R16, sur le modèle de R7), fusion des
  tâches actives et archivées (R18). Une tâche archivée refuse toute modification
  (`DomainError::TaskArchived`) et n'est jamais un enfant affiché (`TaskTree::children`).
- **Affichage** (R15) : `TaskTree::children_with_archives` (archives à leur place, jamais la
  corbeille ; parent à la corbeille → premier niveau) et `ArchiveVisibility` passé à
  `arrange_children`, qui applique la vue de la liste à l'ensemble. `ListTasks` a deux entrées
  (`execute`, `execute_with_archives`) plutôt qu'un paramètre booléen ; `GetTask` et `DeleteTask`
  ne relisent `archive/` que si un identifiant n'est pas parmi les tâches actives
  (`session::tree_containing`).
- **Recherche** (R15) : les fiches de l'index portent leur rayon (`search::Shelf`, `Active` /
  `Archived`) ; `load_archived` indexe les archives, `load_all` ne touche qu'aux fiches actives
  (`SearchIndex::retain_only(shelf, ids)`). Une fiche est à jour si la version **et** le rayon
  correspondent (`is_up_to_date`) : un fichier déplacé à la main entre `tasks/` et `archive/` est
  réindexé. La requête porte `ArchiveVisibility` : sans « Archives », les fiches archivées sont
  écartées. Une archive **chiffrée** est indexée sans sa note : `archive/` n'étant pas relu au
  verrouillage du coffre, sa note déchiffrée resterait sinon en mémoire. Les icônes et le filtre de
  médias des archives viennent de ces fiches.
- **Actions groupées** (sélection multiple, spec §5.5) : les cas d'usage `ArchiveTask`,
  `UnarchiveTask`, `GetArchiveImpact`, `DeleteTask`, `RestoreTask` reçoivent une **liste**
  d'identifiants (`&[TaskId]`) ; une seule tâche est une liste d'un élément. Le domaine les traite
  ancêtres d'abord, chacune une fois (`rules::group::each_once`) : une sous-tâche d'une tâche déjà
  choisie n'est pas traitée deux fois. Une seule date pour le groupe, donc un seul « Annuler » ;
  tout ou rien (la session n'est pas validée en cas d'erreur). La sélection elle-même est un état
  d'affichage (`ui/src/application/treeSelection.ts`).
- **Stockage** : le port `TaskRepository` existant, sans nouveau port. `load_all` ne lit **pas**
  `archive/` ; `load_archived` le lit à la demande (restaurer ; purger ou chiffrer, pour ne pas
  effacer un média qu'une archive cite encore) ; `save` range le fichier selon l'état de la tâche
  et efface l'ancien **après** avoir écrit le nouveau (une coupure laisse un doublon, pas une
  perte). Côté application, `TreeSession::load_with_archives` donne l'arbre complet.
- **Doublon dû à la synchro** (R18) : même identifiant dans `tasks/` et `archive/` → la version la
  plus récente gagne, l'autre va dans `.conflicts/` — sauf si elle ne diffère que par l'archivage
  (copie d'avant que le drive n'a pas encore effacée) : elle est simplement effacée. Réglé par
  `load_all`, qui ne lit que les **noms** de fichiers d'`archive/` pour repérer les doublons.
- **Fichiers « à la demande »** dans `archive/` : lus quand même (la lecture les fait télécharger) ;
  ignorer une archive non téléchargée ferait effacer ses médias à la purge.
- **Anciennes versions** : le premier archivage relève `schema_version` dans `manifest.json` ; une
  version qui ne connaît pas `archive/` refuse la base au lieu d'en effacer des médias.

## Options écartées
- **Un champ `archived_at` dans `tasks/`**, comme la corbeille : plus simple (pas de déplacement),
  mais `tasks/` grossit avec le temps (relu toutes les 3 s) et les archives ne sont pas rangées
  à part, contrairement à la demande.
- **Un dossier d'archives choisi à part** (par exemple hors du drive) : il faudrait y copier les
  médias et y porter le coffre de la base ; reporté tant que le besoin n'existe pas.

## Conséquences
- ➕ Listes et scanner allégés ; archives visibles et rangées sur le disque.
- ➕ Rien n'est perdu : note, médias, vue, chiffrement suivent la tâche.
- ➖ Archiver est un déplacement de fichiers : un conflit de synchro peut laisser deux copies
  (R18 les départage).
- ➖ Mise à jour obligatoire de tous les appareils avant d'archiver dans une base partagée.
- ➖ Une archive chiffrée n'est trouvée que par son titre et ses étiquettes.
