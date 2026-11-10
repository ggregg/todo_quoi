# ADR-0012 — Alignement et couleur du texte des notes

- Statut : Acceptée · Date : 2026-11-10 · Complète l'ADR-0007 (format Markdown étendu)

## Contexte
Le mainteneur veut aligner le texte « là où c'est pertinent » et le colorer. Le Markdown standard
(CommonMark + GFM) ne sait dire ni l'un ni l'autre, sauf l'alignement des colonnes de tableau. La
note doit rester un fichier Markdown lisible, relu sans perte (aller-retour exact sur le corpus).

## Décision
- **Paragraphe aligné** : ligne `{align=center}` (ou `right`, `left`) à la fin du paragraphe.
  **Titre** : `{align=…}` en fin de ligne (`## Titre {align=center}`). Même famille d'écriture
  que `{width=480}` des images (attributs à la Pandoc).
- **Colonne de tableau** : l'alignement **GFM** de la ligne de séparation (`:---:`, `---:`,
  `:---`). Le tableau Markdown aligne par colonne : dans l'éditeur, aligner une cellule aligne
  toute sa colonne. L'alignement écrit est celui de la cellule d'en-tête.
- **Pas d'alignement dans une liste** : un élément de liste « serré » n'a pas de paragraphe au
  relu, l'attribut serait perdu (bouton désactivé, attribut ignoré à l'écriture).
- **Couleur** : `<span data-color="red">texte</span>`, HTML en ligne admis par CommonMark (une
  visionneuse Markdown affiche au moins le texte). **Palette fixe de noms** : `red`, `orange`,
  `green`, `blue`, `purple`, `gray`. Le fichier garde le nom, le thème clair ou sombre choisit la
  teinte (variables `--text-<nom>`), et le Rust refuse tout autre nom ou code couleur.
- **Pas de confusion avec le texte de l'utilisateur** : `{`, `}`, `<`, `>` y sont échappés ; le
  parseur ne reconnaît que la forme **non échappée**, en regardant le source de chaque événement
  (`pulldown-cmark`, `into_offset_iter`).
- **Pas de dépendance** : deux extensions TipTap écrites pour TodoQuoi
  (`ui/src/presentation/editor/formatting.ts`, une quarantaine de lignes) plutôt que
  `@tiptap/extension-text-align`, `-text-style` et `-color`.

## Conséquences
- ➕ La note reste lisible et diffable ; aller-retour exact (corpus `05-alignement-couleurs.md`).
- ➕ Couleurs lisibles dans les deux thèmes ; recherche inchangée (le texte coloré est du texte).
- ➖ Pas de couleur libre ni de surlignage (à ajouter, si besoin, par un nouveau nom de palette ou
  une marque `<mark>`) ; pas de justification.
- ➖ Un autre outil Markdown montrera `{align=center}` tel quel et ignorera les couleurs.
