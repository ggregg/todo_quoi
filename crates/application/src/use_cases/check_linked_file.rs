//! Que peut-on faire d'un fichier lié, ici et maintenant ? (Ouvrir, révéler seulement, rien car
//! créé sur un autre appareil, rien car introuvable.)

use crate::ports::LocalFiles;
use std::path::Path;
use std::sync::Arc;
use tq_domain::{DeviceId, LinkAvailability, availability};

/// Cas d'usage « état d'un fichier lié » (lecture seule).
pub struct CheckLinkedFile {
    files: Arc<dyn LocalFiles>,
    device: DeviceId,
}

impl CheckLinkedFile {
    /// Assemble le cas d'usage ; `device` : cet appareil.
    pub fn new(files: Arc<dyn LocalFiles>, device: DeviceId) -> Self {
        Self { files, device }
    }

    /// Disponibilité du lien `path`, créé sur `link_device`. Le disque n'est lu que si le lien
    /// vient de cet appareil.
    pub fn execute(&self, path: &Path, link_device: &DeviceId) -> LinkAvailability {
        if *link_device != self.device {
            return LinkAvailability::OnOtherDevice(link_device.clone());
        }
        availability(link_device, &self.device, self.files.kind(path))
    }
}
