# POC I0-3 — Fichiers « à la demande » des drives cloud (volet macOS)

> Date : 2026-11-06 · Verdict macOS : **GO** (iCloud Drive + Google Drive) · Windows / OneDrive : **non testé**

## Questions
1. Peut-on **détecter** qu'un fichier n'est pas présent localement, sans le télécharger ?
2. Le **scan périodique** (taille + date de modification) fonctionne-t-il sur un tel fichier sans le rapatrier ?
3. Peut-on **forcer le téléchargement** par l'API et suivre l'état ?
4. Une **simple lecture** suffit-elle à rapatrier le fichier ?

## Montage
Outil en ligne de commande `hydrate` (`src/main.rs`, `src/macos.rs`) :
- détection **sans API** : drapeau `SF_DATALESS` de `stat()` (bibliothèque standard Rust) ;
- état cloud, téléchargement et éviction via Foundation (`objc2-foundation`, 8 classes) :
  `NSURLUbiquitousItemDownloadingStatusKey`, `startDownloadingUbiquitousItemAtURL`, `evictUbiquitousItemAtURL`.

Le scénario ne travaille que sur un fichier qu'il crée dans `<drive>/TodoQuoi-POC/` (conservé).

```bash
cargo build && ./target/debug/hydrate scenario "<dossier du drive>"
```

## Résultats — macOS 27
| Étape | iCloud Drive | Google Drive (File Provider, DriveFS 133) |
|---|---|---|
| Envoi du fichier de test (~200 Ko) | 6,1 s | 17,9 s |
| Éviction → `SF_DATALESS` + statut `NotDownloaded` | ✔ immédiat | ✔ immédiat |
| **Q1** Détection sans téléchargement | ✔ `stat()` suffit | ✔ |
| **Q2** Taille et date exactes, fichier **toujours dataless** après `stat()` | ✔ | ✔ |
| **Q3** `startDownloadingUbiquitousItem` → présent localement | ✔ 1,0 s | ✔ 1,0 s |
| **Q4** `fs::read` d'un fichier évincé → rapatrié, contenu identique | ✔ 1,0 s | ✔ 0,7 s |

La **même API** Foundation sert pour iCloud et pour les drives « File Provider » (Google Drive ;
OneDrive et Dropbox utilisent le même mécanisme sur macOS).

## Constats à reporter (I5, adaptateur `cloud_sync`)
1. **Le scan périodique est compatible** avec les fichiers à la demande : `stat()` ne déclenche pas de téléchargement.
2. **Détection sans Objective-C** : le drapeau `SF_DATALESS` (bibliothèque standard) suffit pour savoir
   qu'un fichier n'est pas local ; Foundation ne sert qu'à **précharger** et à afficher l'état.
3. **Lecture = téléchargement transparent** mais bloquant (~1 s par fichier) : précharger avec
   `startDownloadingUbiquitousItem` les fichiers dataless repérés par le scan, plutôt que de laisser
   la lecture bloquer l'interface.
4. **Accès au dossier Google Drive bloqué** au premier essai (≥ 2 min, sans réponse) avant que l'accès
   soit accordé : **toute E/S sur le drive doit avoir un délai maximal** et s'exécuter hors du fil de l'UI ;
   l'indicateur de synchro doit signaler « drive inaccessible ».
5. Une seule ligne `unsafe` : lecture des constantes `NSURLUbiquitousItem…Key` (documentée `// SAFETY:`).

## Reste à faire — Windows / OneDrive
Cloud Files API (`CfHydratePlaceholder`, attributs `FILE_ATTRIBUTE_RECALL_ON_*`) non testée.
Risque jugé faible (la lecture d'un placeholder OneDrive le rapatrie aussi de façon transparente) :
à vérifier au plus tard en I5, ou plus tôt avec une variante Windows de cet outil.

## Dépendances
`objc2` 0.6, `objc2-foundation` 0.3 (fonctionnalités par défaut désactivées : `NSArray`, `NSDictionary`,
`NSError`, `NSFileManager`, `NSString`, `NSURL`, `NSValue`), uniquement pour la cible macOS.
