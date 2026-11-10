//! Ports du **coffre** (spec §9, ADR-0010) : un coffre par base, créé au premier chiffrement.
//!
//! | Port | Rôle | Adaptateur |
//! |---|---|---|
//! | [`VaultKeys`] | Créer un coffre, retrouver sa clé à partir du mot de passe (lent : Argon2id) | `crypto` |
//! | [`Cipher`] | Chiffrer / déchiffrer des octets avec une clé (rapide) | `crypto` |
//! | [`VaultStore`] | Lire / écrire la description du coffre d'une base (`manifest.json`) | `fs_markdown` |
//! | [`KeyVault`] | « Se souvenir sur cet appareil » : trousseau de l'OS | `keyring` |
//! | [`ContentSealer`] | Ce dont le stockage a besoin : sceller / desceller le contenu d'une tâche de **sa** base | [`crate::vault::BaseSealer`] |

use crate::ports::RepositoryError;
use crate::vault::{ContentKey, Password, VaultError};
use tq_domain::{BaseId, KeyId, SealedContent};

/// Description d'un coffre, écrite **en clair** dans le manifeste de sa base : rien de secret
/// (la clé de contenu n'y est qu'enveloppée, illisible sans le mot de passe).
///
/// Les textes (`salt`, `wrapped_key`) sont produits et relus par l'adaptateur de chiffrement ;
/// l'application et le stockage les transportent sans les interpréter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRecord {
    /// Identifiant du coffre (noté dans chaque tâche chiffrée).
    pub key_id: KeyId,
    /// Dérivation du mot de passe.
    pub kdf: KdfParams,
    /// Sel de la dérivation (hexadécimal).
    pub salt: String,
    /// Clé de contenu chiffrée par la clé dérivée du mot de passe (hexadécimal).
    pub wrapped_key: String,
}

/// Paramètres Argon2id. Écrits dans le manifeste : un coffre créé avec d'autres paramètres
/// (plus forts, un jour) reste ouvrable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    /// Mémoire, en Kio.
    pub memory_kib: u32,
    /// Nombre de passes.
    pub iterations: u32,
    /// Parallélisme.
    pub parallelism: u32,
}

/// Création et ouverture des coffres (dérivation du mot de passe, enveloppe de la clé).
pub trait VaultKeys: Send + Sync {
    /// Crée un coffre : tire une clé de contenu au hasard et l'enveloppe avec `password`.
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`] si l'aléa ou la dérivation échoue.
    fn create(&self, password: &Password) -> Result<(VaultRecord, ContentKey), VaultError>;

    /// Retrouve la clé de contenu d'un coffre.
    ///
    /// # Erreurs
    /// [`VaultError::WrongPassword`] ; [`VaultError::Crypto`] si la description est abîmée.
    fn unlock(&self, record: &VaultRecord, password: &Password) -> Result<ContentKey, VaultError>;

    /// Ré-enveloppe la clé d'un coffre avec un **nouveau** mot de passe (nouveau sel ; mêmes
    /// identifiant et clé de contenu : rien n'est à rechiffrer).
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`] si l'aléa ou la dérivation échoue.
    fn rewrap(
        &self,
        record: &VaultRecord,
        key: &ContentKey,
        password: &Password,
    ) -> Result<VaultRecord, VaultError>;
}

/// Chiffrement authentifié avec une clé de contenu.
pub trait Cipher: Send + Sync {
    /// Chiffre `plaintext`. `associated` (en clair, non stocké) lie le résultat à son contexte :
    /// le même texte chiffré présenté pour une autre tâche ne se déchiffre pas.
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`].
    fn seal(
        &self,
        key: &ContentKey,
        plaintext: &[u8],
        associated: &[u8],
    ) -> Result<String, VaultError>;

    /// Déchiffre ce que [`Cipher::seal`] a produit.
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`] si le texte a été modifié, ou s'il ne vient pas de cette clé ou de
    /// ce contexte.
    fn open(
        &self,
        key: &ContentKey,
        sealed: &str,
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError>;

    /// Comme [`Cipher::seal`], en binaire (médias : pas de texte, deux fois moins de place).
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`].
    fn seal_bytes(
        &self,
        key: &ContentKey,
        plaintext: &[u8],
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError>;

    /// Déchiffre ce que [`Cipher::seal_bytes`] a produit.
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`] (données modifiées, autre clé, autre contexte).
    fn open_bytes(
        &self,
        key: &ContentKey,
        sealed: &[u8],
        associated: &[u8],
    ) -> Result<Vec<u8>, VaultError>;

    /// Empreinte de `content` **avec la clé** (64 chiffres hexadécimaux) : nomme les médias
    /// chiffrés. Deux contenus identiques ont le même nom dans un coffre, mais le nom ne permet
    /// pas de reconnaître un fichier sans la clé.
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`].
    fn fingerprint(&self, key: &ContentKey, content: &[u8]) -> Result<String, VaultError>;
}

/// Description du coffre de chaque base (`manifest.json`).
pub trait VaultStore: Send + Sync {
    /// Coffre de la base ; `None` tant qu'on n'y a rien chiffré.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si le manifeste est illisible.
    fn load_vault(&self, base: &BaseId) -> Result<Option<VaultRecord>, RepositoryError>;

    /// Enregistre le coffre d'une base.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'écriture échoue.
    fn save_vault(&self, base: &BaseId, vault: &VaultRecord) -> Result<(), RepositoryError>;
}

/// Trousseau de l'OS : clés des coffres que l'utilisateur a demandé de garder ouverts sur cet
/// appareil (« Se souvenir sur cet appareil »).
pub trait KeyVault: Send + Sync {
    /// Range la clé du coffre `key_id`.
    ///
    /// # Erreurs
    /// [`VaultError::Keyring`] si le trousseau refuse.
    fn remember(&self, key_id: &KeyId, key: &ContentKey) -> Result<(), VaultError>;

    /// Clé rangée pour ce coffre, s'il y en a une.
    ///
    /// # Erreurs
    /// [`VaultError::Keyring`] si le trousseau est inaccessible.
    fn recall(&self, key_id: &KeyId) -> Result<Option<ContentKey>, VaultError>;

    /// Oublie la clé de ce coffre (absente : rien à faire).
    ///
    /// # Erreurs
    /// [`VaultError::Keyring`] si le trousseau refuse.
    fn forget(&self, key_id: &KeyId) -> Result<(), VaultError>;
}

/// Ce que le stockage d'**une** base demande pour écrire et relire ses tâches chiffrées.
pub trait ContentSealer: Send + Sync {
    /// Chiffre le contenu d'une tâche avec la clé du coffre de la base.
    ///
    /// # Erreurs
    /// [`VaultError::NoVault`] ou [`VaultError::Locked`] : le coffre doit être ouvert.
    fn seal(&self, plaintext: &[u8], associated: &[u8]) -> Result<SealedContent, VaultError>;

    /// Déchiffre ; `None` si le coffre est verrouillé ou si ce n'est pas sa clé (la tâche reste
    /// alors scellée).
    ///
    /// # Erreurs
    /// [`VaultError::Crypto`] si le texte chiffré a été abîmé.
    fn open(
        &self,
        sealed: &SealedContent,
        associated: &[u8],
    ) -> Result<Option<Vec<u8>>, VaultError>;
}
