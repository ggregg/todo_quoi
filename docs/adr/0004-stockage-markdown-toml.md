# ADR-0004 — Un fichier Markdown + TOML par tâche, « dernier qui écrit gagne »

- Statut : Acceptée · Date : 2026-11-06

## Contexte
Les données sont synchronisées par un dossier de drive cloud quelconque, sans serveur. Les fichiers
doivent rester lisibles hors de l'application et indexables par l'OS (sauf tâches chiffrées).

## Décision
- Un fichier par tâche, `tasks/<uuid-v7>.md` : en-tête **TOML entre `+++`** puis la note en Markdown
  étendu. Arborescence plate (hiérarchie portée par le champ `parent`), ordre par indice fractionnaire.
- Conflit entre appareils : **le plus récent gagne** (horodatage hybride), le perdant est conservé
  dans `.conflicts/` pendant 30 jours.
- Les champs d'en-tête inconnus sont **préservés** à la réécriture (compatibilité avec une version future).
- Validé par le POC I0-1 (`poc/i0-1-markdown/RESULTATS.md`).

## Conséquences
- ➕ Fichiers lisibles, éditables, indexés par Spotlight / Windows Search.
- ➕ Un déplacement ou un réordonnancement ne modifie qu'**un** fichier.
- ➖ Deux modifications concurrentes de la même tâche : une seule est gardée (l'autre va dans `.conflicts/`).
- ➖ La crate `toml` réécrit le TOML à sa façon : les tests comparent des valeurs, pas du texte.
