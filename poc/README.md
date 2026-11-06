# Incrément I0 — POC des risques : rapport go/no-go

> Date : 2026-11-06 · Branche `poc/i0` · **Verdict global : GO pour I1**

Les POC sont du **code jetable** : ils répondent à une question, puis servent de référence.
Le code de production sera réécrit proprement dans l'architecture hexagonale (I1+).

| POC | Question | Verdict | Détails |
|---|---|---|---|
| **I0-1** Markdown | Une note riche passe-t-elle par TipTap et revient-elle en Markdown + TOML sans perte ? | **GO** | [`i0-1-markdown/RESULTATS.md`](i0-1-markdown/RESULTATS.md) |
| **I0-2** Médias | Audio et images traitables dans la webview, sans dépendance, lisibles sur les deux OS ? | **GO** macOS + Windows | [`i0-2-medias/RESULTATS.md`](i0-2-medias/RESULTATS.md) |
| **I0-3** Drives | Fichiers « à la demande » détectables, scannables, rapatriables ? | **GO** macOS · Windows non testé | [`i0-3-hydratation/RESULTATS.md`](i0-3-hydratation/RESULTATS.md) |

## Décisions prises grâce aux POC
1. **Audio : format unique WebM/Opus** à 32 kbit/s (au lieu de WebM ou MP4 selon l'OS) — spec §6.2.
2. **Images : JPEG 85** (PNG si transparence) ; WebP écarté car non encodable dans WKWebView.
3. **Scan périodique confirmé** : `stat()` ne rapatrie pas les fichiers à la demande ; détection par
   `SF_DATALESS` sans Objective-C ; préchargement par l'API Foundation.
4. **Délai maximal sur toute E/S drive**, hors du fil de l'UI (accès Google Drive bloqué > 2 min observé).

## Corrections et points d'attention pour l'implémentation
| Où | Point | Incrément |
|---|---|---|
| Rendu Markdown | Sortir les espaces des bords de gras / italique / barré (bug trouvé et corrigé au POC) | I2 |
| Tests | Comparer des **valeurs** TOML, pas du texte (`.120Z` → `.12Z`) | I1 |
| Directives | Reconnues seulement au premier niveau ; alignement de tableau non conservé | I2 |
| Audio | Indicateur « enregistrement » à `onstart` (début coupé de 0,7 s sur Mac), durée stockée dans la note | I3 |
| Images | Compression dans un Worker (`OffscreenCanvas`), jusqu'à 3 s sur le fil principal | I3 |
| `tq-media://` | Renvoyer un vrai 404 (Vite renvoie `index.html`) | I3 |
| `cloud_sync` | Préchargement des fichiers dataless repérés par le scan ; état « drive inaccessible » | I5 |

## Reste ouvert
- **OneDrive sous Windows** (Cloud Files API) : non testé. Risque jugé faible ; à vérifier au plus tard
  en I5, ou plus tôt avec une variante Windows de `i0-3-hydratation`.
- Les fichiers de test `TodoQuoi-POC/` créés dans iCloud Drive et Google Drive peuvent être supprimés.

## Dépendances réellement utilisées (toutes dans l'inventaire, architecture §16)
- Rust : `pulldown-cmark`, `serde`, `serde_json`, `toml`, `tauri`, `tauri-build`, `objc2`, `objc2-foundation`.
- npm : `@tiptap/*` (6 paquets), `@tauri-apps/api`, `@tauri-apps/cli`, `vite`, `typescript`.
- Aucune dépendance audio, image, base de données ou surveillance de fichiers.
