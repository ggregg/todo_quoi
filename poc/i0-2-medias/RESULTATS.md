# POC I0-2 — Audio et images dans la webview (sans dépendance)

> Date : 2026-11-06 · Verdict macOS : **GO** · Windows : **à faire** (lecture croisée)

## Question
Peut-on enregistrer / relire l'audio et compresser les images **dans la webview**, sans
crate Rust ni bibliothèque JS, et avec des fichiers lisibles sur les deux OS ?

## Montage
App Tauri minimale (`src-tauri/` : deux commandes, `report` et `save_sample`) + page TypeScript
(`src/main.ts`) qui exécute les tests au chargement ; le test micro se lance par un bouton.
Rapport écrit dans `$TMPDIR/todoquoi-poc-i0-2.json`. Échantillons partagés : `public/samples/`.

```bash
pnpm install && pnpm tauri dev
```

## Résultats — macOS 27, WKWebView (AppleWebKit/605.1.15)

### Audio
| Test | Résultat |
|---|---|
| `MediaRecorder` disponible | ✔ |
| Formats acceptés | `webm;codecs=opus` ✔ · `webm` ✔ · `mp4;codecs=mp4a.40.2` ✔ · `mp4` ✔ · `ogg;codecs=opus` ✘ |
| Format par défaut | MP4/AAC |
| Enregistrement + relecture MP4/AAC (tonalité) | ✔ `decodeAudioData` et `<audio>` |
| **Enregistrement forcé WebM/Opus** + relecture | ✔ (mais `<audio>.duration = NaN` juste après l'enregistrement) |
| **Lecture du WebM/Opus produit par Chromium 152** (≈ WebView2) | ✔ |
| Micro : autorisation macOS + enregistrement + relecture | ✔ (en `tauri dev`, Info.plist intégré au binaire) |

### Images
| Test | Résultat |
|---|---|
| Encodage WebP par `toBlob` | ✘ (renvoie du PNG) → **JPEG confirmé** |
| Orientation EXIF appliquée (`createImageBitmap`, `imageOrientation: 'from-image'`) | ✔ 300×150 → 150×300 |
| Après compression : orientation source et GPS supprimés | ✔ (WebKit ajoute un EXIF minimal de 76 o : un pointeur vide, sans orientation) |
| Transparence → PNG, opaque → JPEG | ✔ |
| 6000×4000 → 2560×1707 | ✔ 821 Ko → 126 Ko ; **152 ms puis 3 177 ms** selon les passages |

## Constats à reporter dans l'implémentation (I3)
1. **Format audio unique possible : WebM/Opus**, puisque WKWebView l'enregistre et le lit, et que
   Chromium le produit nativement. À confirmer sous Windows ; si oui, mettre à jour la spec §6.2
   (un seul format au lieu de WebM ou MP4).
2. **Imposer le débit** : `new MediaRecorder(stream, { mimeType, audioBitsPerSecond: 32000 })`.
   Par défaut : ~1 Mo/min (AAC micro) et 37 Ko pour 1,5 s (Opus WKWebView).
3. **Début d'enregistrement coupé** : 3 s demandées → 2,2–2,3 s obtenues. N'indiquer « enregistrement »
   qu'à l'événement `onstart` (et/ou démarrer le recorder dès que le flux est prêt).
4. **Durée audio** : un WebM fraîchement enregistré n'expose pas sa durée → la mesurer pendant
   l'enregistrement et la stocker dans `::audio{duration=…}` (déjà prévu par la spec).
5. **Compression d'image hors fil principal** (`OffscreenCanvas` dans un Worker) : jusqu'à 3 s observées.
6. Le serveur Vite renvoie `index.html` pour un fichier absent : en production, le protocole
   `tq-media://` devra renvoyer un vrai 404.

## Reste à faire — Windows (WebView2)
Lancer le même POC sur le PC Windows : il lira automatiquement `tonalite-macos.m4a`,
`tonalite-macos-force.webm`, `micro-macos.m4a` et produira `tonalite-windows.webm` / `micro-windows.webm`
pour la vérification inverse sur Mac.

## Dépendances
- Rust : `tauri`, `tauri-build`, `serde`, `serde_json` (gabarit `tauri init` allégé de `log` et `tauri-plugin-log`).
- npm : `@tauri-apps/api` ; dev : `vite`, `typescript`, `@tauri-apps/cli`.
- Aucune dépendance audio ou image.
