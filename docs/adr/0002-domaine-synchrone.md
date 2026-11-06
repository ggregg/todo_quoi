# ADR-0002 — Domaine et application synchrones

- Statut : Acceptée · Date : 2026-11-06

## Contexte
L'asynchrone Rust (`async`/`await`, exécuteurs, `Send`/`Pin`, durées de vie dans les futures) est
l'une des parties les plus difficiles du langage. Les E/S de TodoQuoi sont des lectures / écritures
de petits fichiers locaux, rapides ; aucune requête réseau n'est faite par l'application elle-même.

## Décision
- `domain`, `application` et les adaptateurs exposent des fonctions **synchrones**.
- La couche Tauri exécute chaque cas d'usage dans un fil de travail (`spawn_blocking`) pour ne
  jamais bloquer l'interface ; les E/S sur le drive ont un délai maximal (constat du POC I0-3).

## Conséquences
- ➕ Code nettement plus simple à lire et à déboguer pour un débutant Rust.
- ➕ Tests sans exécuteur asynchrone.
- ➖ Si un jour un adaptateur parle à une API réseau (drive par API, V2), il devra gérer son propre
  asynchronisme en interne, derrière le même port synchrone.
