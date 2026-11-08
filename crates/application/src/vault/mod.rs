//! Le **coffre** côté application (spec §9, ADR-0010) : secrets, coffres ouverts, et le
//! « scelleur » que reçoit le stockage de chaque base.
//!
//! - [`Password`], [`ContentKey`] : secrets, effacés de la mémoire à leur libération ;
//! - [`OpenVaults`] : clés des coffres déverrouillés, **en mémoire seulement**, par base ;
//! - [`BaseSealer`] : implémente le port `ContentSealer` pour une base ;
//! - [`VaultError`] : ce qui peut mal tourner.

mod base_sealer;
mod error;
mod open_vaults;
mod secrets;

pub use base_sealer::BaseSealer;
pub use error::VaultError;
pub use open_vaults::{OpenVaults, VaultState};
pub use secrets::{ContentKey, MIN_PASSWORD_CHARS, Password, Remember};
