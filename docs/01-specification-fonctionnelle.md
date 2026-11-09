# TodoQuoi — Spécification fonctionnelle (V1)

> Statut : **brouillon à valider** · Version 0.1 · 2026-11-05

## 1. Vision

Gestionnaire de tâches **client lourd**, mono-utilisateur, multi-appareils, **local-first**.
Chaque tâche peut contenir des sous-tâches (profondeur illimitée) et une **note riche**.
Les données vivent dans un dossier d'un **drive cloud quelconque** (OneDrive, Google Drive, iCloud Drive, Dropbox…) qui assure le transport entre appareils.

**Cibles** : V1 Windows 11 + macOS · ensuite Linux · peut-être iOS.

## 2. Périmètre

### Dans la V1
- Liste arborescente des tâches, zoom dans une tâche, présentation par liste (colonnes / tri / filtres) héritable.
- CRUD tâche, statut binaire, tags, corbeille.
- Proposition de clôture du parent / réouverture du parent.
- Note riche en blocs : texte formaté, liens, images, cases à cocher, groupes radio, tableaux, blocs de code, pièces jointes, messages audio (enregistrés dans l'app).
- Recherche plein texte globale.
- Navigation clavier complète (raccourcis classiques).
- Panneau de détail latéral, détachable en fenêtre.
- Synchronisation via dossier de drive, aides OS pour forcer le téléchargement.
- Chiffrement optionnel par tâche.
- Interface FR, i18n prête, thèmes clair / sombre / système.
- Versionnage de schéma + migrations.

### Hors V1 (prévu plus tard)
Échéances, priorités, récurrence, rappels/notifications, historique de versions interne, import/export, transcription audio, palette de commandes, raccourcis personnalisables, mode vim, signature de code / mise à jour auto, Linux, iOS.

## 3. Modèle de tâche

| Champ | Type | Règle |
|---|---|---|
| `id` | UUID v7 | Immuable, généré à la création (triable chronologiquement). |
| `parent` | UUID v7 \| absent | absent = racine. |
| `order` | chaîne (indice fractionnaire) | Position parmi les frères ; un déplacement ne réécrit qu'**un** fichier. |
| `title` | texte | Obligatoire, une ligne. |
| `done` | booléen | Statut binaire : **Ouvert** / **Terminé**. |
| `done_at` | date-heure \| null | Renseigné à la clôture. |
| `tags` | liste de textes | Libres, insensibles à la casse pour la recherche, saisie avec autocomplétion. |
| `created_at`, `updated_at` | date-heure UTC | `updated_at` sert d'arbitre « dernier qui écrit gagne ». |
| `updated_by` | id d'appareil | Diagnostic. |
| `deleted_at` | date-heure \| null | Non nul = dans la corbeille. |
| `encrypted` | booléen | Note chiffrée (cf. §9). |
| `view` | objet \| absent | Présentation de la liste de ses sous-tâches (cf. §5.3). |
| note | Markdown étendu | Corps du fichier. |

**Champs calculés (non stockés)** : progression `n terminées / m sous-tâches`, nombre de médias, présence d'audio / pièces jointes.

## 4. Règles métier

### 4.1 Clôture
- **R1** — Quand la **dernière** sous-tâche ouverte d'un parent est clôturée, l'outil propose : *« Toutes les sous-tâches de « X » sont terminées. Clôturer « X » ? »* [Clôturer] [Plus tard]. Si le parent est clôturé et que le grand-parent devient à son tour entièrement terminé, la proposition remonte avec **une boîte par niveau** ; « Plus tard » arrête la remontée.
- **R2** — Clôturer un parent qui a encore des sous-tâches ouvertes : demande *« Clôturer aussi les N sous-tâches ouvertes ? »* [Toutes] [Seulement le parent] [Annuler].
- **R3** — Rouvrir une sous-tâche dont le parent est terminé : propose *« Rouvrir aussi « X » (et ses ancêtres terminés) ? »* [Rouvrir] [Non].
- **R4** — « Plus tard » ne repropose pas tant que l'état des enfants ne change pas.

### 4.2 Suppression / corbeille
- **R5** — Supprimer = mettre `deleted_at` sur la tâche **et tous ses descendants** ; toast « Annuler » 8 s.
- **R6** — La corbeille (bouton « Corbeille » sous la liste) liste les racines supprimées (sous-arbres repliés), avec date et chemin d'origine ; « Restaurer » applique R7.
- **R7** — Restaurer remet le sous-arbre à sa place ; si le parent d'origine n'existe plus ou est lui-même en corbeille, restauration à la racine (avec message).
- **R8** — Purge définitive : manuelle, tâche par tâche (« Supprimer définitivement » dans la corbeille, avec confirmation ; emporte ses sous-tâches à la corbeille ; jamais une tâche active), ou automatique après **30 jours**. La purge supprime les fichiers sur tous les appareils (synchronisation), **avec les médias** (images, audios, pièces jointes) que plus aucune autre note de la base ne cite — sauf si une note de la base est verrouillée (coffre fermé) : on ne peut pas savoir ce qu'elle cite, ces médias partiront à une purge ultérieure. À venir : « Vider la corbeille » en une fois, durée réglable.

### 4.3 Déplacement / hiérarchie
- **R9** — Indenter (`Tab`) : la tâche devient dernière enfant du frère précédent. Désindenter (`Maj+Tab`) : devient frère suivant de son parent.
- **R10** — Glisser-déposer dans l'arbre (avant / après / dans : quart haut, quart bas, milieu de la ligne). Lâcher sur le fond de la liste ou sur un élément du fil d'Ariane place la tâche à la fin de cette liste (« Accueil » = premier niveau : détacher).
- **R11** — Interdit de déplacer une tâche dans son propre sous-arbre.
- **R12** — Le déplacement ne recalcule pas les statuts mais déclenche R1/R3 sur l'ancien et le nouveau parent.

## 5. Interface

### 5.1 Écran principal
```
┌──────────────────────────────────────────────────────────────────────┐
│ ☰  Accueil › Projet A › Lot 2        [🔍 Rechercher…]     ⟳ ● 🌓 ⚙ │
├──────────────────────────────┬───────────────────────────────────────┤
│ Filtres: [Ouvertes ▾] [tags ▾]│  ☐ Titre de la tâche          ⧉  ✕   │
│ ▾ ☐ Tâche 1          2/5 #a   │  #tag1 #tag2   + tag                  │
│   ▸ ☐ Sous-tâche 1.1  0/3     │  ─────────────────────────────────── │
│     ☑ Sous-tâche 1.2          │  Note (éditeur en blocs)             │
│ ▸ ☐ Tâche 2          🔒       │  …                                    │
│ + Nouvelle tâche              │  Sous-tâches (2/5)  ↗ zoomer          │
├──────────────────────────────┴───────────────────────────────────────┤
│ 1 248 tâches · synchronisé il y a 3 s · OneDrive/TodoQuoi            │
└──────────────────────────────────────────────────────────────────────┘
```
- **Barre haute** : fil d'Ariane (zoom), recherche, indicateur de synchro, thème, réglages.
- **Liste** (gauche) : arbre dépliable, virtualisé ; colonnes configurables.
- **Panneau détail** (droite) : redimensionnable, masquable, **détachable** (`⧉`) en fenêtre indépendante.
- **Fenêtre de tâche** : une tâche peut s'ouvrir **seule dans sa propre fenêtre système** (cf. §5.4bis) ; plusieurs fenêtres simultanées.
- **Barre d'état** : volume, état de synchro, dossier du drive.

### 5.1bis Bases (ADR-0009)
- Une **base** = un dossier `TodoQuoi/` : « Perso », « Pro »… Elle peut être **n'importe où** — disque local, disque externe, partage réseau ou drive cloud ; l'outil détecte le stockage, l'affiche et s'adapte (fichiers à la demande d'un drive, disque débranché, dossier qui ne répond pas). Chaque base a un nom et une couleur, choisis sur l'appareil.
- **Bandeau** sous la barre haute : « Tout » (toujours présent) et une pastille par base. Clic sur une base → elle seule ; clic sur « Tout » → toutes, chacune en **section** de premier niveau (dépliable, à sa couleur). Les dossiers restent séparés ; seul l'affichage les réunit. Le choix est mémorisé sur l'appareil.
- Menu d'une pastille (clic droit ou « ⋯ ») : renommer l'étiquette, changer la couleur, **détacher** la base (retirée de l'outil, dossier intact).
- Bouton « + » du bandeau : fenêtre « Bases » — ajouter une base avec la **fenêtre native de choix de dossier** (choisir un dossier existant pour rattacher une base, ou en créer un nouveau ; drives détectés et chemin collé aussi proposés ; nom proposé d'après le dossier, couleur), renommer, changer la couleur, retirer de l'app (le dossier n'est ni effacé ni modifié). Ajouter une base **conserve** les autres, y compris la base locale de départ « Mes tâches ».
- Créer : au premier niveau d'une base (bouton « Ajouter » de sa section, ou de la base affichée seule).
- Déplacer une tâche **vers une autre base** (glisser sur une tâche ou sur la section de cette base) : la tâche part avec ses sous-tâches et leurs médias ; elle disparaît de la base d'origine.
- Recherche, corbeille : limitées aux bases affichées ; chaque résultat indique sa base.
- Chaque base a sa propre vue racine (§5.3) et sa propre synchronisation (§7) ; l'indicateur résume toutes les bases.

### 5.2 Liste
- Ligne : case à cocher, titre, progression `n/m` si enfants, tags, icônes (🔒 chiffrée, 🖼 image, 🎙 audio, 📎 pièce jointe, 📝 note non vide), date de modification.
- Tâches terminées : barrées, atténuées ; visibles ou masquées selon filtre.
- Édition inline du titre : `F2` ou clic lent.
- **Zoom** : afficher uniquement les sous-tâches d'une tâche (la tâche devient la racine affichée) ; fil d'Ariane cliquable pour remonter.

### 5.3 Présentation par liste (`view`)
Chaque liste (racine ou tâche zoomée) peut définir :
- **colonnes** visibles et leur ordre : titre (fixe), progression, tags, icônes, créée le, modifiée le, terminée le ;
- **tri** : manuel (ordre `order`, défaut), titre, créée, modifiée, terminée, progression ; multi-critères ;
- **filtres** : statut (toutes / ouvertes / terminées), tags (inclus / exclus), « contient médias », texte.

**Héritage** : une liste sans `view` propre utilise celle de l'ancêtre le plus proche qui en a une, sinon la vue racine. Bouton « Personnaliser cette liste » (crée une surcharge) / « Revenir à l'héritage ».
L'état **déplié/replié** des nœuds est local à l'appareil (non synchronisé).

### 5.4 Panneau de détail
- Titre éditable, case statut, tags (chips + autocomplétion), bouton 🔒 chiffrer/déchiffrer.
- Éditeur de note en blocs (cf. §6).
- Section **Sous-tâches** : mini-liste (ajout rapide, cocher), bouton « Zoomer ».
- Méta : créée / modifiée / par quel appareil.
- Sauvegarde automatique (débounce 1 s, flush à la perte de focus / fermeture).

### 5.4bis Fenêtre de tâche
- **Ouverture** : `Maj+Entrée`, `Maj+double-clic`, clic droit › « Ouvrir dans une nouvelle fenêtre », ou bouton `⧉` / `Mod+Maj+D` depuis le panneau (la tâche quitte alors le panneau).
- Contenu identique au panneau de détail ; titre de la fenêtre = titre de la tâche.
- Une fenêtre **par tâche** : rouvrir une tâche déjà ouverte en fenêtre ramène cette fenêtre au premier plan.
- Bouton `⇤` : réancrer la tâche dans le panneau latéral ; `Échap` ou fermeture système : ferme la fenêtre.
- Modifications reflétées en temps réel dans la liste et dans les autres fenêtres.
- Si la tâche est supprimée (ici ou par synchro), sa fenêtre se ferme ; position et taille mémorisées par tâche (local à l'appareil) ; fenêtres rouvertes au redémarrage.

### 5.5 Raccourcis clavier (Mod = Ctrl sous Windows/Linux, ⌘ sous macOS)

| Action | Raccourci |
|---|---|
| Se déplacer | `↑` `↓`, `Début` `Fin`, `PgPréc` `PgSuiv` |
| Déplier / replier / aller au parent | `→` / `←` |
| Ouvrir dans le panneau | `Entrée` ou double-clic |
| Ouvrir dans une nouvelle fenêtre | `Maj+Entrée` ou `Maj+double-clic` (ou clic droit) |
| Zoomer dans la tâche / remonter | `Mod+Entrée` / `Mod+Retour arrière` (ou `Alt+←`) |
| Clôturer / rouvrir | `Espace` |
| Nouvelle tâche (frère suivant) | `Mod+N` |
| Nouvelle sous-tâche | `Mod+Maj+N` |
| Renommer | `F2` |
| Indenter / désindenter | `Tab` / `Maj+Tab` |
| Monter / descendre | `Alt+↑` / `Alt+↓` |
| Supprimer (corbeille) | `Suppr` |
| Rechercher | `Mod+F` |
| Annuler / rétablir | `Mod+Z` / `Mod+Maj+Z` |
| Fermer le panneau / la fenêtre de tâche | `Échap` |
| Détacher la tâche en fenêtre | `Mod+Maj+D` |
| Passer liste ↔ panneau | `F6` |
| Réglages | `Mod+,` |

Sélection multiple : `Maj+↑/↓`, `Mod+clic` ; actions groupées : clôturer, supprimer, tagger.

### 5.6 Recherche
- Instantanée (≤ 100 ms sur 10 000 tâches), sur titre, tags, note (y compris texte des tableaux, cases, radios, noms de pièces jointes).
- Insensible à la casse et aux accents.
- Syntaxe : mots, `"phrase exacte"`, `tag:client-x`, `-tag:perso`, `is:open` / `is:done`, `has:audio|image|file`, `in:"Projet A"` (sous-arbre).
- Résultats : liste plate avec chemin (fil d'Ariane) et extrait surligné ; `Entrée` ouvre, `Mod+Entrée` révèle dans l'arbre.
- Tâches chiffrées : titre et étiquettes toujours cherchables ; la **note** seulement si le coffre est déverrouillé (index en mémoire, jamais sur disque — cf. architecture).

### 5.7 Réglages
Dossier du drive, langue, thème, purge corbeille (jours), compression des images (qualité, taille max, conserver l'original), coffre (phrase de passe, verrouillage auto après N min), appareil (nom : **repris du nom de la machine** au premier lancement, modifiable ici).

## 6. Notes — éditeur en blocs

Saisie WYSIWYG, raccourcis Markdown à la frappe, menu `/` pour insérer un bloc, poignée de glisser pour réordonner.

| Bloc | Saisie | Stockage Markdown |
|---|---|---|
| Paragraphe, gras, italique, barré, code inline | `**`, `_`, `~~`, `` ` `` | standard |
| Titres H1–H3 | `# `, `## `, `### ` | standard |
| Listes à puces / numérotées | `- `, `1. ` | standard |
| Citation | `> ` | standard |
| Lien | `Mod+K` ou bouton (texte + adresse ; sans sélection, le lien est inséré), collage ou frappe d'une URL | `[texte](url)` ; barre « Ouvrir / Modifier / Retirer » quand le curseur est sur le lien, `Mod+clic` ouvre. Seuls `http`, `https` et `mailto` s'ouvrent (navigateur ou messagerie du système) ; `www.…` est complété en `https://` |
| Lien vers une tâche | `[[` + recherche | `[titre](tq://task/<id>)` |
| **Cases à cocher** | `[] ` | `- [ ] item` / `- [x] item` (GFM) |
| **Groupe radio** | `/radio` | directive `:::radio` (cf. ci-dessous) |
| Tableau | `/tableau` | table GFM |
| Bloc de code | ```` ``` ```` + langage | bloc clôturé, police à chasse fixe (coloration syntaxique après V1) |
| **Image** | coller, glisser, `/image` | `![alt](../media/<hash>.jpg){width=480}` |
| **Pièce jointe** | glisser un fichier, `/fichier` | `::file{src="../media/<hash>.pdf" name="devis.pdf" size=48213}` |
| **Message audio** | `/audio` ou `Mod+Maj+R` | `::audio{src="../media/<hash>.webm" duration=12.4}` |
| Séparateur | `---` | standard |

Groupe radio (un seul choix) :
```markdown
:::radio{label="Environnement"}
- ( ) Développement
- (x) Recette
- ( ) Production
:::
```

### 6.1 Images
- Coller depuis le presse-papiers ou glisser-déposer (fichier ou depuis navigateur).
- Compression auto : côté long max **2560 px**, **JPEG qualité 85** (PNG si l'image a de la transparence) ; GIF animés et SVG conservés tels quels ; métadonnées EXIF supprimées (orientation appliquée) ; option « conserver l'original » (stocké à côté).
- Redimensionnement visuel par poignée (largeur stockée), clic = visionneuse plein écran.

### 6.2 Audio
- Bouton enregistrer dans la note : niveau d'entrée, durée, pause, stop, annuler.
- Format unique **WebM/Opus**, mono, 32 kbit/s (~0,25 Mo/min), enregistré et lu à l'identique sous Windows et macOS (validé au POC I0-2).
- La durée est mesurée pendant l'enregistrement et stockée dans la note (les fichiers WebM ne l'exposent pas toujours) ; l'indicateur « enregistrement » ne s'allume qu'une fois le micro réellement démarré.
- Lecteur intégré : lecture/pause, barre de position, vitesse ×1 / ×1,5 / ×2, durée.
- Choix du micro dans les réglages ; demande d'autorisation micro gérée par l'OS.

### 6.3 Pièces jointes
- Taille max par fichier : **50 Mo** (réglable) ; avertissement au-delà de 10 Mo.
- Ouvrir avec l'application par défaut, « Révéler dans le Finder/Explorateur », « Enregistrer sous… ».

## 7. Synchronisation (vue utilisateur)
- Bases : voir §5.1bis (fenêtre « Bases », `Mod+,` ou « + » du bandeau). Ajouter ou détacher une base recharge les fenêtres, sans redémarrer l'application. Le choix des bases est propre à chaque appareil.
- Indicateur (barre haute) : ● vert synchronisé · ● bleu clignotant téléchargement en cours (fichiers « à la demande ») · ● orange à vérifier (fichier illisible, conflit réglé non consulté) · ● rouge drive inaccessible. Un clic montre le détail et le journal des conflits ; le retour de la fenêtre au premier plan relance un passage.
- Modifications d'un autre appareil : apparaissent automatiquement (scan du dossier toutes les 3 s) ; si la tâche ouverte est modifiée ailleurs pendant l'édition, bandeau « Modifiée sur *MacBook* — [Recharger] [Garder la mienne] ».
- Conflits : règle **dernier qui écrit gagne** (sur `updated_at`, puis nom de l'appareil à la milliseconde près) ; la version perdante est déplacée dans `.conflicts/` (filet de sécurité, purgé après 30 jours) et signalée dans le journal de l'indicateur. Copies de conflit créées par le drive (`<id> (1).md`, `… (conflicted copy …).md`) : réglées de la même façon.
- Vidage automatique : corbeille et `.conflicts/` de plus de 30 jours, au démarrage puis une fois par jour.

## 8. Versionnage du schéma
- `schema_version` dans `manifest.json` et dans chaque fichier tâche.
- Au démarrage : si les fichiers sont plus anciens → migration automatique (fichier par fichier, idempotente, au fil de l'eau) ; si le manifeste annonce une version **plus récente** que celle de l'app → **lecture seule** + message « Mettez à jour TodoQuoi sur cet appareil ».
- Chaque appareil publie sa version d'app et de schéma (`.devices/`), ce qui permet d'avertir avant une migration qui bloquerait un appareil plus ancien.

## 9. Chiffrement optionnel par tâche
- **Un coffre par base** (ADR-0010), créé à la première utilisation : le premier chiffrement dans une base demande d'en choisir le mot de passe (saisi deux fois ; avertissement : **perte = données irrécupérables**). Chaque coffre se déverrouille et se verrouille seul, verrouillage auto configurable. « Se souvenir sur cet appareil » (décoché par défaut) range la clé dans le trousseau de l'OS (Trousseau macOS / Gestionnaire d'identification Windows).
- Déplacer une tâche chiffrée vers une autre base la rechiffre avec le coffre de la base cible : les deux coffres doivent être déverrouillés.
- Bouton 🔒 sur une tâche : chiffre **la note** (et ses médias) de la tâche ; le titre, les étiquettes, la vue, le statut et la place restent en clair (choix du mainteneur, ADR-0010). Option « chiffrer aussi les sous-tâches » (proposée par défaut).
- Coffre verrouillé : la tâche apparaît normalement dans l'arbre, avec un cadenas ; son titre, ses étiquettes et son statut restent modifiables ; sa note est masquée (« Déverrouiller pour la lire ») et ne peut pas être modifiée.
- Notes chiffrées **illisibles par l'OS** (Spotlight / Windows Search n'indexent que le titre et les étiquettes) ; les autres tâches le sont entièrement (fichiers `.md`).

## 10. Exigences non fonctionnelles
| Exigence | Cible |
|---|---|
| Démarrage à froid (5 000 tâches, cache présent) | < 1,5 s |
| Reconstruction complète du cache (5 000 tâches) | < 10 s |
| Défilement liste | 60 i/s, liste virtualisée |
| Recherche | < 100 ms |
| Propagation entre appareils | délai du drive + ≤ 3 s après réception (scan périodique) |
| Perte de données | aucune écriture partielle (écriture atomique), perdant de conflit conservé |
| Accessibilité | navigation 100 % clavier, contrastes AA, lecteurs d'écran (rôles ARIA tree/treeitem) |
| Données | aucune télémétrie, aucun appel réseau en dehors du drive |

## 11. Décisions tranchées (2026-11-06)
1. R1 : **une boîte par niveau** lors de la remontée de clôture.
2. Conservation corbeille et `.conflicts/` : **30 jours** (réglable).
3. Pièces jointes : **50 Mo** max par fichier (réglable), avertissement au-delà de 10 Mo.
4. Nom de l'appareil : **nom de la machine** par défaut, modifiable dans les réglages.

Aucun point ouvert à ce jour.
