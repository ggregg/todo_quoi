# ADR-0014 — Retrait des groupes de choix (« radio ») des notes

- Statut : Acceptée · Date : 2026-11-24 · Modifie l'ADR-0007 (directives du Markdown étendu)

## Contexte
Le bloc « groupe de choix » (un seul choix possible, directive `:::radio`), livré en I3, ne sert
pas : le mainteneur demande de le retirer définitivement. Des notes existantes peuvent encore en
contenir, sur n'importe quel appareil synchronisé.

## Décision
- Le bloc disparaît de l'éditeur (bouton, nœuds TipTap `radioGroup` / `radioOption`), de la
  spécification et du format écrit : `:::radio` n'est **plus jamais écrit**.
- **Rien n'est perdu** : un ancien groupe est encore **lu** (`note_markdown`, `directives.rs`) et
  devient son libellé, en paragraphe, suivi d'une liste de cases ; l'option choisie est cochée.
  À la prochaine sauvegarde de la note, le fichier est en Markdown standard
  (`- [x] Pré-production`).
- La recherche trouve toujours le libellé et les options.

## Conséquences
- ➕ Moins de code (un nœud, sa vue et son greffon de sélection), un format plus proche du
  Markdown standard.
- ➖ La relecture des anciens groupes reste dans `directives.rs` : on pourra la retirer quand plus
  aucune note n'en contiendra (aucune migration de fichiers n'est faite).
- ➖ Une note relue mais pas modifiée garde `:::radio` dans son fichier jusqu'à sa prochaine
  sauvegarde.
