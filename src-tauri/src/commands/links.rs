//! Liens des notes : ouverture dans le navigateur ou la messagerie du système.

use crate::error::UiError;
use crate::system_open::open_with_system;
use std::ffi::OsStr;
use tq_application::AppError;
use tq_domain::ExternalLink;

/// Ouvre un lien externe (`http`, `https`, `mailto` seulement). `invoke('open_link', { url })`.
///
/// La webview ne navigue jamais elle-même vers un lien de note : elle quitterait l'application.
#[tauri::command]
pub fn open_link(url: String) -> Result<(), UiError> {
    let link = ExternalLink::parse(&url).map_err(|error| UiError::from(AppError::from(error)))?;
    open_with_system(OsStr::new(link.as_str()))
}
