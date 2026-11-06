# ADR-0001 — Architecture hexagonale en crates séparées

- Statut : Acceptée · Date : 2026-11-06

## Contexte
Le code doit rester lisible et modifiable par un mainteneur débutant en Rust, testable sans
disque ni interface, et ouvert à de nouveaux stockages (drive par API en V2) ou plateformes (iOS).

## Décision
- Quatre couches, chacune dans **sa propre crate Cargo** :
  `domain` (règles métier pures) ← `application` (cas d'usage + ports) ← `adapters/*` (technique)
  ← `src-tauri` (adaptateurs primaires + composition root).
- Une crate ne déclare que les dépendances vers les couches plus centrales : **le compilateur
  interdit** à `domain` d'importer `tauri`, `toml` ou le système de fichiers.
- Les **ports** sont des traits définis dans `application` ; les adaptateurs les implémentent.

## Conséquences
- ➕ Métier et cas d'usage testés en millisecondes avec des fakes (`crates/testing`).
- ➕ Changer de stockage = écrire un adaptateur ; le reste ne bouge pas.
- ➖ Plus de fichiers et de conversions (domaine ↔ fichier ↔ DTO) qu'une application « tout-en-un ».
- ➖ Il faut résister à la tentation de mettre une règle métier dans un adaptateur ou dans l'UI.
