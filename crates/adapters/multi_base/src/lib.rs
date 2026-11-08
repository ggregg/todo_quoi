//! # Plusieurs bases ouvertes ensemble (ADR-0009)
//!
//! **Place dans l'hexagone** : adaptateur secondaire « composite ». Chaque base est un dossier
//! `TodoQuoi/` avec ses propres adaptateurs (dépôt, médias, vue racine, scan) ; ce module les
//! présente comme **un seul** port aux cas d'usage, en aiguillant chaque opération :
//! - [`MultiBaseTaskRepository`] (`TaskRepository`) : la base d'une tâche décide de son dossier ;
//!   un changement de base = écrire dans la nouvelle, copier les médias cités, effacer l'ancienne ;
//! - [`MultiBaseMediaStore`] (`MediaStore`) : stocker dans une base, lire dans n'importe laquelle
//!   (un média est nommé par son empreinte : même nom = même contenu) ;
//! - [`MultiBaseRootViews`] (`RootViewStore`), [`MultiBaseVaults`] (`VaultStore`, un coffre par
//!   base), [`MultiBaseDriveSync`] (`DriveSync`) ;
//! - [`OpenBases`] (`BaseDirectory`) : les bases ouvertes, dans l'ordre du bandeau.
//!
//! Une seule base ouverte : les composites ne font que transmettre.

mod drive;
mod media;
mod repository;
mod views;

pub use drive::MultiBaseDriveSync;
pub use media::MultiBaseMediaStore;
pub use repository::MultiBaseTaskRepository;
pub use views::{MultiBaseRootViews, MultiBaseVaults, OpenBases};

use tq_application::ports::RepositoryError;
use tq_domain::BaseId;

/// Erreur « base inconnue » (fermée entre-temps, ou fichier modifié à la main).
pub(crate) fn unknown_base(base: &BaseId) -> RepositoryError {
    RepositoryError::Unavailable(format!("base « {base} » non ouverte sur cet appareil"))
}

/// L'adaptateur de la base `base` dans une liste `(base, adaptateur)`.
pub(crate) fn find<'a, T: ?Sized>(
    bases: &'a [(BaseId, std::sync::Arc<T>)],
    base: &BaseId,
) -> Result<&'a std::sync::Arc<T>, RepositoryError> {
    bases
        .iter()
        .find(|(known, _)| known == base)
        .map(|(_, adapter)| adapter)
        .ok_or_else(|| unknown_base(base))
}
