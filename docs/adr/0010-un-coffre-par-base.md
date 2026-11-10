# ADR-0010 — Un coffre par base, créé par un mot de passe à la première utilisation

- Statut : Acceptée · Date : 2026-11-08 · Complète la spec §9 et l'ADR-0009 (plusieurs bases)

## Contexte
La spec §9 prévoit un **coffre** : les tâches marquées 🔒 sont chiffrées, illisibles sans la
phrase de passe. Depuis l'ADR-0009, plusieurs bases sont ouvertes ensemble (perso, pro…),
chacune dans son propre dossier, souvent partagé ou synchronisé différemment. Décision du
mainteneur : **chaque base a son propre coffre**, déverrouillé par un mot de passe, et ce mot de
passe est **choisi à la première utilisation** (premier chiffrement dans cette base).

## Décision
- **Un coffre par base**, décrit dans le `manifest.json` de la base (section `vault`) : sel,
  paramètres Argon2id, identifiant du coffre (`key_id`), clé de contenu **enveloppée**. Il
  voyage donc avec le dossier : un autre appareil qui ouvre la base y trouve le même coffre
  et le même mot de passe.
- **Création à la première utilisation** : chiffrer une tâche d'une base sans coffre demande
  de choisir un mot de passe (saisi deux fois, avertissement « perte = données
  irrécupérables »). Aucune base n'a de coffre tant qu'on n'a rien chiffré.
- **Clés** : une clé de contenu aléatoire de 256 bits chiffre les tâches et médias
  (XChaCha20-Poly1305). Elle est enveloppée par une clé dérivée du mot de passe (Argon2id,
  m = 64 Mio, t = 3, p = 1). Changer de mot de passe = ré-envelopper la clé de contenu, sans
  rechiffrer les tâches. Un mauvais mot de passe est détecté par l'échec de l'authentification
  de l'enveloppe (pas de condensat du mot de passe stocké).
- **Verrouillage indépendant** : chaque coffre se déverrouille et se verrouille seul (pastille
  de la base), verrouillage automatique après N minutes d'inactivité. La clé déverrouillée ne
  vit qu'en mémoire (`zeroize` à la libération).
- **Se souvenir sur cet appareil** (case à cocher, décochée par défaut) : la clé de contenu est
  rangée dans le trousseau de l'OS (Trousseau macOS, Gestionnaire d'identification Windows),
  une entrée par coffre (`key_id`). C'est un réglage de l'appareil, jamais écrit dans la base.
- **Déplacer une tâche chiffrée vers une autre base** : elle est déchiffrée avec la clé de la
  source et rechiffrée avec celle de la cible (coffre créé à ce moment si besoin) ; les deux
  coffres doivent être déverrouillés, sinon le déplacement est refusé avec un message.
- **Seule la note est chiffrée** (décision du mainteneur après essai, 2026-11-08) : le titre,
  les étiquettes, la vue, le statut et la place restent en clair — la liste reste lisible et
  organisable coffre verrouillé ; le secret, c'est la note (et ses médias).
- **Fichier d'une tâche chiffrée** : toujours `tasks/<id>.md` (pas de `.tqe` binaire comme
  l'envisageait l'architecture §4.2 — le scan, les conflits et les doublons restent le même
  code). L'en-tête TOML est inchangé (plus `key_id`) ; le corps — la note — est remplacé par son
  texte chiffré. `schema_version = 2` : une version qui ignore le chiffrement écarte le fichier
  au lieu de l'écraser. Les tâches en clair restent en version 1.
- **Données associées** (AAD) = `todoquoi-task:<id>` : un texte chiffré recopié dans une autre
  tâche ne se déchiffre pas. L'en-tête n'en fait **pas** partie, pour qu'une tâche scellée
  puisse être renommée, close ou déplacée coffre verrouillé : seul l'en-tête est réécrit, la
  note chiffrée est recopiée telle quelle.
- **Tâche scellée** (coffre verrouillé) dans le domaine : `Confidentiality::Sealed` — modifier
  la note, la remettre en clair ou changer de base sont **refusés** (on écraserait la vraie
  note par une note vide) ; tout le reste est permis. La recherche la trouve par son titre et
  ses étiquettes, pas par sa note.
- **Ports** : `VaultKeys` (créer / ouvrir un coffre, Argon2id), `Cipher` (XChaCha20-Poly1305),
  `VaultStore` (manifeste), `KeyVault` (trousseau) et `ContentSealer` (ce que le stockage d'une
  base demande, implémenté par `BaseSealer` à partir des coffres ouverts `OpenVaults`). Les
  coffres ouverts survivent aux rechargements de l'assemblage (ajout d'une base).

## Conséquences
- ➕ Les bases restent indépendantes : partager la base « Pro » ne livre pas le mot de passe
  « Perso », et perdre l'un n'expose que sa base.
- ➕ Pas de coffre, pas de dépendance au trousseau ni de question tant qu'on ne chiffre rien.
- ➖ Le titre et les étiquettes d'une tâche chiffrée sont lisibles par quiconque accède au
  dossier : ne pas y mettre de secret (assumé, dit dans la fenêtre de création du coffre).
- ➖ Un mot de passe par base à retenir ; « Se souvenir sur cet appareil » limite la saisie.
- ➖ Mot de passe oublié = notes chiffrées de cette base irrécupérables (assumé, signalé à la
  création).
- ➖ Deux appareils qui créent le coffre d'une même base **au même moment** (avant que le drive
  n'ait synchronisé le manifeste) : le dernier manifeste écrit gagne, et les tâches chiffrées
  par l'autre coffre restent scellées (`key_id` inconnu). Localement, un coffre n'est jamais
  remplacé par un autre (`AlreadyExists`) ; le cas croisé, très improbable, n'est pas traité.

## Réalisé en I6
- Domaine, cas d'usage, adaptateurs `crypto` et `keyring`, format de fichier, coffre dans le
  manifeste, commandes, interface (cadenas des pastilles, bouton 🔒 / 🔓, dialogue de mot de
  passe, note scellée).
- **Médias chiffrés** (2026-11-10) : `media/<HMAC>.<ext>.tqe`, contenu XChaCha20-Poly1305 en
  binaire, AAD = nom (architecture §4.2). Le HMAC utilise une **sous-clé de nommage** dérivée de
  la clé de contenu (une clé, un usage). Chiffrer / déchiffrer une tâche renomme ses médias et
  efface les anciennes copies que plus rien ne cite (règle `uncited_media`, même prudence que la
  purge si une note de la base est scellée) ; une tâche chiffrée qui change de base voit ses
  médias rechiffrés avec la clé de la destination. Un média chiffré se lit avec n'importe quel
  coffre ouvert (seule la bonne clé l'authentifie) ; « Ouvrir » passe par une copie
  déchiffrée temporaire (`src-tauri/src/plain_copies.rs`), effacée au verrouillage et au
  lancement.
- **Verrouillage automatique** : délai de l'appareil (15 min par défaut, `0` = jamais), activité
  signalée par l'interface, minuteur dans `src-tauri`. Les coffres gardés dans le trousseau en
  sont exemptés : l'utilisateur a choisi de faire confiance à l'appareil, et ils se rouvriraient
  seuls au lancement.
- **Changement de mot de passe** : ré-enveloppe la clé (nouveau sel, paramètres Argon2 actuels),
  sans rechiffrer.
- **Déplacement vers une base au coffre fermé** : refusé d'avance avec un message explicite
  (`VaultError::TargetLocked`), au lieu d'un échec à l'écriture.
- **Fenêtres de tâche détachées** : mêmes boutons ; toutes les fenêtres suivent l'état des
  coffres (événement `vaults-changed`).

## Reste à faire
- Déplacer une tâche chiffrée vers une base **sans** coffre : refusé (même message) ; proposer
  de créer le coffre de la destination puis reprendre le déplacement.
- Une note chiffrée dans une base où un autre coffre a écrit (création simultanée, cf.
  Conséquences) : non traité.
