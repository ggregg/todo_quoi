//! Identité de cet appareil, tirée du nom de la machine (spécification : « repris du nom de la
//! machine », modifiable ensuite dans les réglages).

use tq_domain::DeviceId;

/// Identifiant de cet appareil : nom de la machine normalisé (`MacBook-Pro-de-Greg.local` →
/// `macbook-pro-de-greg`), ou `appareil` si le nom est introuvable.
///
/// Sources, dans l'ordre : la commande `hostname` (macOS, Linux), puis les variables
/// d'environnement `COMPUTERNAME` (Windows) et `HOSTNAME` (Linux).
pub fn machine_device_id() -> DeviceId {
    let name = hostname_command()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_default();
    DeviceId::from_machine_name(without_domain(&name))
}

/// Sous Windows, pas de commande : lancer un programme console (`hostname.exe`) depuis une app
/// graphique fait clignoter une fenêtre de console. `COMPUTERNAME` donne le même nom.
#[cfg(windows)]
fn hostname_command() -> Option<String> {
    None
}

#[cfg(not(windows))]
fn hostname_command() -> Option<String> {
    let output = std::process::Command::new("hostname").output().ok()?;
    let name = String::from_utf8(output.stdout).ok()?;
    let name = name.trim();
    (output.status.success() && !name.is_empty()).then(|| name.to_string())
}

/// `MacBook-Pro.local` → `MacBook-Pro` (le suffixe réseau n'apporte rien à l'utilisateur).
fn without_domain(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_suffix_is_dropped() {
        assert_eq!(
            without_domain("MacBook-Pro-de-Greg.local"),
            "MacBook-Pro-de-Greg"
        );
        assert_eq!(without_domain("PC-BUREAU"), "PC-BUREAU");
    }

    #[test]
    fn this_machine_has_a_valid_device_id() {
        let id = machine_device_id();
        assert!(DeviceId::parse(id.as_str()).is_ok(), "{id}");
    }
}
