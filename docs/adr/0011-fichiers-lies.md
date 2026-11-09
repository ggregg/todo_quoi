# ADR-0011 — Joindre, déplacer ou lier un fichier ; liens propres à un appareil

- Statut : Acceptée · Date : 2026-11-09 · Complète la spec §6 (blocs de la note)

## Contexte
Jusqu'ici, un fichier ajouté à une note était toujours **copié** dans la base (`media/`). Le
mainteneur veut aussi pouvoir le **déplacer** (ranger le fichier avec la note et le retirer de son
dossier) ou le **lier** (laisser le fichier où il est). Une base est synchronisée sur plusieurs
appareils : un chemin n'a de sens que sur la machine qui l'a lié, et le fichier peut disparaître.

## Décision
- **Bouton 📎 « Joindre »** : fenêtre native de choix de fichiers (plugin `dialog`, permission
  `dialog:allow-open` déjà accordée), puis un choix unique pour tous les fichiers choisis :
  Copier (par défaut), Déplacer, Lier. Glisser-déposer et coller restent des **copies** : la
  webview ne donne pas le chemin d'un fichier déposé.
- **Déplacer** = copier dans la base, puis effacer l'original — seulement si la copie a réussi.
- **Lier** : directive `::filelink{path name device}` — chemin tel que l'OS d'origine l'écrit,
  nom affiché, appareil (`DeviceId`, nom de la machine). Rien n'est copié ; un lien n'est pas un
  média (ni copié vers une autre base, ni effacé par la purge).
- **Disponibilité** (règle du domaine, `tq_domain::availability`) : lien d'un autre appareil →
  visible, inactif, sans même regarder le disque ; fichier absent → « introuvable » ; dossier ou
  programme → **révéler seulement** ; document → ouvrir et révéler. Le cœur revérifie à chaque
  ouverture (`open_linked_file`, `reveal_linked_file`).
- **Sécurité** : une note est synchronisée, donc modifiable depuis un autre poste. Ouvrir un
  dossier (une application macOS `.app` en est un) ou un programme depuis une note pourrait le
  lancer : c'est refusé. « Programme » = extension d'exécutable, de script ou d'installeur, ou
  fichier exécutable sans extension (le bit `x` seul ne suffit pas : sur un disque externe ou un
  partage réseau, tous les fichiers l'ont souvent).
- **Ports** : `LocalFiles` (genre d'un chemin, lire, effacer — adaptateur `system::FsLocalFiles`),
  cas d'usage `AttachFile`, `LinkFile`, `CheckLinkedFile`.

## Conséquences
- ➕ Pas de copie inutile pour les gros fichiers qui ont déjà leur place (plans, vidéos…).
- ➕ Un lien venu d'un autre poste ne s'ouvre jamais par erreur sur un fichier homonyme.
- ➖ Un lien ne suit pas un fichier renommé ou déplacé : il devient « introuvable » (le chemin
  reste visible au survol pour le retrouver).
- ➖ Le nom de l'appareil est celui de la machine : renommer la machine rend ses liens
  « d'un autre appareil ».

## Alternatives écartées (glisser-déposer)

Poser la même question au glisser-déposer demanderait le chemin du fichier déposé :

- **Gestionnaire de dépôt de Tauri** (`dragDropEnabled`) : il donne les chemins, mais intercepte
  *tous* les glisser-déposer de la webview (macOS et Windows) — arbre, barre des bases, fil
  d'Ariane et texte de la note ne se déplaceraient plus.
- **Code natif par OS** (presse-papiers de glisser-déposer `NSPasteboard` sous macOS,
  `postMessageWithAdditionalObjects` de WebView2 sous Windows) : faisable (~150 lignes), mais
  deux dépendances directes (`objc2-app-kit`, `webview2-com`) et du code `unsafe` hors de
  `cloud_sync`. Écarté pour l'instant (Greg, 2026-11-09) : le bouton 📎 couvre le besoin.
