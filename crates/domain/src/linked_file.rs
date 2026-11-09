//! Fichier **lié** depuis une note : la note ne contient pas le fichier, seulement son chemin
//! sur la machine qui l'a lié (`::filelink{path name device}`). Les autres appareils voient le
//! lien, inactif ; la machine d'origine l'ouvre tant que le fichier existe.
//!
//! Le domaine ne lit pas le disque : l'adaptateur dit ce qu'il trouve au chemin
//! ([`LocalFileKind`]), et [`availability`] décide ce qu'on peut faire du lien.

use crate::DeviceId;

/// Ce qui se trouve à un chemin, sur cette machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFileKind {
    /// Rien (fichier déplacé, renommé, effacé, disque débranché…).
    Missing,
    /// Un dossier — y compris une application macOS (`.app` est un dossier).
    Directory,
    /// Un programme (exécutable, script, installeur…).
    Program,
    /// Un document ordinaire.
    Document,
}

/// Ce que permet un lien, ici et maintenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAvailability {
    /// Ouvrir (application par défaut) et révéler (Finder / Explorateur).
    Available,
    /// Révéler seulement : ouvrir un dossier ou un programme venu d'une note synchronisée
    /// pourrait lancer une application.
    RevealOnly,
    /// Lien créé sur un autre appareil : visible, inactif.
    OnOtherDevice(DeviceId),
    /// Le fichier n'est plus là.
    Missing,
}

/// Disponibilité d'un lien créé sur `link_device`, vu depuis `this_device`, où le chemin désigne
/// `kind`. Sur un autre appareil, le chemin n'est même pas regardé : il n'y désigne rien de
/// fiable (un fichier homonyme serait un autre fichier).
pub fn availability(
    link_device: &DeviceId,
    this_device: &DeviceId,
    kind: LocalFileKind,
) -> LinkAvailability {
    if link_device != this_device {
        return LinkAvailability::OnOtherDevice(link_device.clone());
    }
    match kind {
        LocalFileKind::Missing => LinkAvailability::Missing,
        LocalFileKind::Directory | LocalFileKind::Program => LinkAvailability::RevealOnly,
        LocalFileKind::Document => LinkAvailability::Available,
    }
}

/// Un fichier lié : chemin sur l'appareil d'origine, nom affiché, appareil.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedFile {
    /// Chemin absolu, tel que l'OS d'origine l'écrit.
    pub path: String,
    /// Nom du fichier (affiché dans la note, cherchable).
    pub name: String,
    /// Appareil qui a créé le lien.
    pub device: DeviceId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str) -> DeviceId {
        DeviceId::parse(name).unwrap()
    }

    #[test]
    fn availability_table() {
        let (here, other) = (device("mac-greg"), device("pc-bureau"));
        let cases = [
            (&here, LocalFileKind::Document, LinkAvailability::Available),
            (&here, LocalFileKind::Missing, LinkAvailability::Missing),
            (
                &here,
                LocalFileKind::Directory,
                LinkAvailability::RevealOnly,
            ),
            (&here, LocalFileKind::Program, LinkAvailability::RevealOnly),
            (
                &other,
                LocalFileKind::Document,
                LinkAvailability::OnOtherDevice(other.clone()),
            ),
            (
                &other,
                LocalFileKind::Missing,
                LinkAvailability::OnOtherDevice(other.clone()),
            ),
        ];
        for (link_device, kind, expected) in cases {
            assert_eq!(availability(link_device, &here, kind), expected, "{kind:?}");
        }
    }
}
