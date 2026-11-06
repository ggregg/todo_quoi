# POC I0-1 — Aller-retour TipTap ↔ Markdown étendu + front-matter TOML

> Date : 2026-11-06 · Verdict : **GO**

## Question
Peut-on stocker une note riche en Markdown (lisible, indexable par l'OS) et la faire passer
par l'éditeur TipTap **sans perte**, dans les deux sens ?

## Montage
```
fichier .md ──(Rust : mdconv to-json)──► JSON ProseMirror ──(TipTap : fromJSON + check + toJSON)──►
JSON normalisé ──(Rust : mdconv to-md)──► fichier .md   ⇒ identique à l'original ?
```
- Rust (`src/`) : `pulldown-cmark` + directives maison (`:::radio`, `::audio`, `::file`) + en-tête TOML `+++`.
- TypeScript (`editor/`) : schéma TipTap (StarterKit, image, tableau, liste de tâches + 4 nœuds maison), validation en Node sans navigateur.

## Lancer
```bash
cargo test
```
```bash
cargo build && cd editor && pnpm install && pnpm validate
```

## Résultats
| Vérification | Résultat |
|---|---|
| Aller-retour Markdown exact (Rust seul), 4 fichiers de corpus | ✔ |
| JSON produit **strictement valide** pour le schéma TipTap (`node.check()`) | ✔ |
| Aller-retour exact **en passant par TipTap** (attributs par défaut ajoutés : `target`, `rel`, `height: null`…) | ✔ |
| Édition par transaction ProseMirror (case cochée, choix radio) reflétée dans le Markdown | ✔ |
| En-tête TOML typé : champs inconnus (`couleur`, `[view]`) conservés | ✔ |
| clippy `-D warnings`, rustfmt | ✔ |

Blocs couverts : paragraphes, titres, gras / italique / barré / code / liens (y compris imbriqués),
listes à puces / numérotées (départ ≠ 1) / imbriquées, cases à cocher (avec sous-liste), groupe radio,
tableau, bloc de code (une directive à l'intérieur reste du code), citation multi-paragraphes,
séparateur, saut de ligne forcé, image avec titre et largeur, audio, pièce jointe, échappements.

## Constats à reporter dans l'implémentation (I1 / I2)
1. **Espaces en bord de marque** : `**gras **suite` n'est pas du gras en CommonMark. Trouvé pendant le POC,
   **corrigé** (`render::expel_edge_whitespace`) + test de non-régression.
2. **TOML réécrit sous sa forme à lui** : `11.120Z` → `11.12Z` (même valeur), `sort = [{…}]` → `[[view.sort]]`.
   Sans conséquence (l'app régénère l'en-tête) ; les tests doivent comparer des **valeurs**, pas du texte.
3. Limites assumées, à traiter en I2 si besoin :
   - directives reconnues seulement au premier niveau (pas dans une liste / citation) ;
   - première ligne d'un tableau toujours écrite comme en-tête ; alignement des colonnes non conservé ;
   - retour à la ligne « souple » (sans `\`) relu comme un espace ;
   - soulignement désactivé dans l'éditeur (pas d'équivalent Markdown) ;
   - échappement prudent (`a\_b`) : correct mais parfois plus verbeux que nécessaire.
4. Le modèle JSON générique (`PmNode` avec `kind: String`) convient pour l'échange ; le domaine aura un
   `enum Block` typé, et la conversion JSON ↔ domaine sera dans `src-tauri/dto`.

## Dépendances utilisées (toutes dans l'inventaire §16)
- Rust : `pulldown-cmark` (sans fonctionnalités par défaut), `serde`, `serde_json`, `toml` → ~23 crates au total, pur Rust.
- npm : `@tiptap/core`, `@tiptap/pm`, `@tiptap/starter-kit`, `@tiptap/extension-image`,
  `@tiptap/extension-table`, `@tiptap/extension-list` (3.32.3) → 44 paquets au total.
