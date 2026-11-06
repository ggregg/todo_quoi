+++
schema_version = 1
id = "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e"
parent = "0195f3c0-1a2b-7c3d-8e4f-5a6b7c8d9e0f"
order = "a0V"
title = "Préparer la release 1.2"
done = false
tags = ["release", "client-x"]
created_at = 2026-11-05T16:02:11.120Z
updated_at = 2026-11-05T17:25:12.345Z
updated_by = "mac-greg"
encrypted = false
couleur = "rouge"

[view]
columns = ["title", "progress", "tags", "updated_at"]
sort = [{ field = "manual" }]
filter = { status = "open", tags_in = [], tags_out = [] }
+++
# Release 1.2

Objectif : livrer la **1.2** avant le *15 novembre*, sans ~~régression~~ majeure.

Voir le [ticket EXT-412](https://jira.exemple.fr/browse/EXT-412) et la commande `cargo test`.

Un **gras avec un [lien](https://exemple.fr) dedans** et du ***gras italique***.

## Étapes

- Préparer le changelog
- Recette
  - Scénarios SSO
  - Export **PDF**
- Déploiement

### Ordre de passage

1. Front
2. API
3. Base de données
