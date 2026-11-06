# ADR-0007 — La note reste du Markdown dans le domaine ; conversion à la frontière

- Statut : Acceptée · Date : 2026-11-06 · Remplace la partie « `Note { blocks }` » de l'architecture §2.1

## Contexte
L'architecture prévoyait un modèle de note en blocs typés dans le domaine (`enum Block`), avec deux
conversions : JSON TipTap → domaine (couche Tauri) et domaine → Markdown (stockage).
Or **aucune règle métier** ne lit aujourd'hui le contenu d'une note : le domaine se contente de la
stocker. Le POC I0-1 a validé une conversion directe Markdown étendu ↔ JSON TipTap, sans perte.

## Décision
- Le domaine garde la note en **texte Markdown** (`NoteBody`), sans la comprendre.
- La conversion Markdown étendu ↔ JSON TipTap est un **adaptateur** : la crate
  `crates/adapters/note_markdown`, reprise du POC I0-1 (`pulldown-cmark` + directives maison).
- Elle est appelée **à la frontière** (`src-tauri/dto`) : le JSON de l'éditeur est converti en
  Markdown avant d'entrer dans l'application, et inversement à la lecture.

## Conséquences
- ➕ Une seule conversion, déjà testée ; moins de code et de types à maintenir.
- ➕ Le domaine reste minimal et lisible.
- ➖ Le jour où une règle aura besoin du contenu (recherche plein texte I4, icônes « contient audio »),
  il faudra soit un extrait calculé par l'adaptateur (texte brut, présence de médias), soit
  introduire le modèle en blocs — par une nouvelle ADR.
