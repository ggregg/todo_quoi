# ADR-0006 — Politique de dépendances

- Statut : Acceptée · Date : 2026-11-06

## Contexte
Les projets du mainteneur contiennent des données clients ; chaque dépendance est une surface
d'attaque, une charge de maintenance et une chose de plus à comprendre.

## Décision
- Une dépendance n'entre que si elle fait ce qu'on ne peut pas écrire **simplement et sûrement**
  (crypto, parseur CommonMark, API système, éditeur riche), ou si elle n'ajoute rien au binaire.
- Elle est d'abord ajoutée, avec sa justification, à l'**inventaire** (architecture §16), puis validée
  par le mainteneur ; versions communes déclarées dans `[workspace.dependencies]`.
- Écrits à la main : UUID v7, dates RFC 3339, horloge hybride, ordre fractionnaire, index de recherche.
- Revue à chaque incrément : `cargo tree --depth 1`, `pnpm list --depth 1`.

## Conséquences
- ➕ Environ 11 crates directes et 8 paquets npm d'exécution pour toute la V1.
- ➕ Le code « maison » est court, testé et documenté (lecture pédagogique).
- ➖ Un peu plus de code à maintenir qu'en important une bibliothèque pour chaque besoin.
