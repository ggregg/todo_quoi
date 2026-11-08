# ADR-0008 — Synchronisation par le dossier du drive sans dépendance système

- Statut : Acceptée, **révisée le 2026-11-07** (sélecteur natif) · Modifie l'architecture §5.4 et l'inventaire §16

## Contexte
L'I5 rend l'application utilisable sur plusieurs appareils via un dossier de drive. L'architecture
prévoyait :
- un sélecteur de dossier natif, qui impose le plugin `tauri-plugin-dialog` (crate + paquet npm) ;
- un adaptateur `cloud_sync` appuyé sur `objc2` / `objc2-foundation` (macOS) et `windows` (Windows)
  pour précharger les fichiers « à la demande » (iCloud, File Provider, Cloud Files).

Le POC I0-3 a montré que la bibliothèque standard suffit pour **détecter** un fichier non
téléchargé (`SF_DATALESS` dans `stat()` sous macOS ; attributs `RECALL_ON_*` sous Windows), et
qu'une simple lecture le **rapatrie** — en bloquant, d'où la nécessité de la faire hors du fil de
l'interface et avec un délai maximal.

## Décision
- **Choix du dossier** : pas de sélecteur natif. Un écran de réglage propose les dossiers de drive
  détectés (iCloud, Google Drive, OneDrive, Dropbox ; macOS et Windows) et accepte un chemin saisi
  ou collé, vérifié par le Rust. Le choix est enregistré **sur l'appareil** (réglages locaux, non
  synchronisés) ; l'application redémarre sur le nouveau dossier.
- **Fichiers à la demande** : bibliothèque standard seulement. Détection par les métadonnées,
  rapatriement par une lecture en tâche de fond avec délai maximal ; l'indicateur affiche
  « en cours » tant que des fichiers attendent.
- `objc2`, `objc2-foundation`, `windows` et `tauri-plugin-dialog` passent dans « Écarté » de
  l'inventaire (§16), avec la condition qui les ferait revenir.

## Conséquences
- ➕ Aucune dépendance ajoutée, aucun code `unsafe` (la règle `unsafe` réservée à `cloud_sync`
  devient sans objet).
- ➕ Même code de détection sur macOS et Windows, testable sans drive (fichiers ordinaires).
- ➖ Pas d'épinglage « toujours garder sur cet appareil » proposé par l'application ; l'utilisateur
  le règle dans son client de drive s'il le souhaite.
- ➖ Pas de vrai sélecteur graphique : il faut coller un chemin si le dossier n'est pas détecté.
- ↩ À revoir si : un drive rend des fichiers illisibles sans préchargement natif (constaté sur
  OneDrive / Windows, encore non vérifié), ou si la saisie de chemin se révèle trop pénible.

## Révision (2026-11-07) — sélecteur de dossier natif
À l'usage, coller un chemin s'est révélé trop pénible et le mainteneur veut pouvoir **choisir et
créer** le dossier d'une base depuis une fenêtre. On ajoute donc le plugin officiel
`tauri-plugin-dialog` (+ `@tauri-apps/plugin-dialog`), seule façon de montrer la fenêtre de choix
du système (Finder, Explorateur) avec son bouton « Nouveau dossier ». Les drives détectés et la
saisie d'un chemin restent proposés. Le reste de la décision (fichiers à la demande en
bibliothèque standard) est inchangé.
