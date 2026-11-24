# ADR-0013 — Taille du texte des notes

- Statut : Acceptée · Date : 2026-11-24 · Complète l'ADR-0012 (alignement et couleur du texte)

## Contexte
Le mainteneur veut changer la taille du texte d'une note **sur une sélection** (un mot, une
phrase), comme la couleur, et non régler la taille d'affichage de toutes les notes. Le Markdown
standard ne sait pas le dire ; la note doit rester un fichier Markdown lisible, relu sans perte.

## Décision
- **Écriture** : `<span data-size="large">texte</span>`, HTML en ligne admis par CommonMark, de la
  même famille que `<span data-color="…">` (ADR-0012).
- **Liste fixe de noms** : `small`, `large`, `huge` ; sans balise, la taille est normale. Le
  fichier garde le nom, l'interface choisit la valeur (`0.85em`, `1.25em`, `1.6em`, relative au
  bloc : un « grand » dans un titre reste plus grand que le titre). Le Rust refuse tout autre nom
  ou toute taille en pixels.
- **Couleur et taille ensemble** : deux balises imbriquées, la couleur à l'extérieur
  (`<span data-color="red"><span data-size="large">…</span></span>`) ; `</span>` ferme la
  dernière ouverte.
- **Pas de dépendance** : une marque TipTap `textSize` écrite pour TodoQuoi
  (`ui/src/presentation/editor/formatting.ts`), à côté de `textColor`.

## Conséquences
- ➕ Même mécanique que la couleur : aller-retour exact (corpus `06-tailles.md`), recherche
  inchangée (le texte agrandi reste du texte).
- ➕ Une taille illisible ou démesurée ne peut pas entrer dans un fichier.
- ➖ Pas de taille libre ; un autre outil Markdown ignore les tailles (il affiche au moins le texte).
