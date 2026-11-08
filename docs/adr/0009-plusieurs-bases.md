# ADR-0009 — Plusieurs bases de tâches ouvertes ensemble

- Statut : Acceptée · Date : 2026-11-07 · Complète ADR-0004 (stockage) et ADR-0008 (dossier)

## Contexte
Un dossier `TodoQuoi/` (manifeste, `tasks/`, `media/`) est une **base** de tâches. L'utilisateur
veut en ouvrir plusieurs à la fois (perso, pro…), souvent dans des drives différents, et passer
en un clic de l'affichage d'une seule base à l'affichage de toutes. Choix validés avec le
mainteneur : en mode « Tout », **une section par base** ; une tâche peut être **déplacée d'une
base à l'autre avec ses médias** ; la recherche porte sur **les bases affichées**.

## Décision
- **Stockage indifférent** : une base peut être sur le disque de la machine, un disque externe,
  un partage réseau ou un drive cloud. Le code est le même (scan, délais, fichiers à la demande
  ignorés tant qu'absents) ; le type de stockage est seulement **détecté** par le chemin
  (`system::classify_folder`) pour l'afficher et adapter les messages.
- **Domaine** : nouveau type `BaseId`. Chaque tâche appartient à une base (`Task::base`), qui
  n'est **pas écrite** dans le fichier : c'est le dossier qui la porte. Règle : une tâche et son
  parent sont dans la même base ; déplacer une tâche vers une autre base y emmène tout son
  sous-arbre (R11 inchangée : jamais de cycle).
- **Application** : la base apparaît là où l'utilisateur la choisit — créer au premier niveau
  d'une base, déplacer vers une base, vue racine d'une base, recherche limitée à des bases.
  Les autres cas d'usage ne changent pas : ils passent par les ports habituels.
- **Adaptateurs** : une crate `adapters/multi_base` assemble N bases en **composites** qui
  implémentent les ports existants (`TaskRepository`, `MediaStore`, `RootViewStore`,
  `DriveSync`) en aiguillant chaque opération vers la bonne base. Un changement de base au
  `save` = écrire dans la nouvelle, copier les médias cités par la note, effacer l'ancienne.
  Les médias étant nommés par leur empreinte, une copie ne crée jamais de doublon.
- **Réglages** : la liste des bases (nom, couleur, dossier) est un réglage **de l'appareil**
  (`settings.json`), comme le dossier unique de l'ADR-0008, qu'elle remplace (migration :
  l'ancien dossier devient la base « Mes tâches »). Les bases affichées sont un état d'interface.
- **Interface** : bandeau sous la barre haute — une pastille par base et « Tout » ; clic sur une
  base = elle seule, clic sur « Tout » = toutes, en sections.

## Conséquences
- ➕ Domaine et cas d'usage existants presque inchangés ; une seule base = comportement actuel.
- ➕ Chaque base garde son propre dossier, donc son propre partage et sa propre synchronisation.
- ➖ Déplacer entre deux drives n'est pas atomique : écrire d'abord dans la base cible, puis
  effacer la source ; une coupure entre les deux laisse un doublon (même `id`), que la règle
  « dernier qui écrit gagne » résout au chargement suivant (jamais de perte).
- ➖ Le chiffrement (I6) devra choisir : un coffre par base ou commun — à trancher en I6.
