//! Protocole `tq-media://` : sert les médias à l'interface (images, audio de l'éditeur).
//!
//! L'interface ne lit **jamais** directement le disque : elle demande
//! `tq-media://localhost/<nom>` (macOS, Linux) ou `http://tq-media.localhost/<nom>` (Windows) ;
//! ce module lit le média via le cas d'usage `ReadMedia` et le renvoie avec le bon type.
//!
//! Les requêtes partielles (`Range: bytes=…`) sont gérées : le lecteur audio de WKWebView en a
//! besoin pour se déplacer dans un fichier. Un média absent renvoie un vrai 404 (constat du
//! POC I0-2 : Vite renvoyait la page d'accueil à la place).

use crate::composition::{AppState, LiveState};
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

/// Nom du protocole (à déclarer aussi dans la CSP de `tauri.conf.json`).
pub const SCHEME: &str = "tq-media";

/// Point d'entrée enregistré dans `lib.rs` : traite la requête dans un fil de travail.
pub fn handle<R: Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = match app.try_state::<LiveState>() {
            Some(live) => respond(&live.current(), &request),
            None => plain(
                StatusCode::SERVICE_UNAVAILABLE,
                "application en cours de démarrage",
            ),
        };
        responder.respond(response);
    });
}

fn respond(state: &AppState, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let name = request.uri().path().trim_start_matches('/');
    let (media, bytes) = match state.read_media.execute(name) {
        Ok(found) => found,
        Err(error) => return plain(StatusCode::NOT_FOUND, &error.to_string()),
    };
    let content_type = content_type(media.extension());
    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_range(value, bytes.len()));
    let builder = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header("X-Content-Type-Options", "nosniff");
    let result = match range {
        Some((start, end)) => builder
            .status(StatusCode::PARTIAL_CONTENT)
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{}", bytes.len()),
            )
            .body(bytes[start..=end].to_vec()),
        None => builder.status(StatusCode::OK).body(bytes),
    };
    result.unwrap_or_else(|error| plain(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()))
}

/// `bytes=100-199` → (100, 199) ; `bytes=100-` → jusqu'à la fin. Plage invalide → `None`
/// (on renvoie alors le fichier entier, ce que les navigateurs acceptent).
fn parse_range(value: &str, length: usize) -> Option<(usize, usize)> {
    let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
    let last = length.checked_sub(1)?;
    let start: usize = start.trim().parse().ok()?;
    let end: usize = if end.trim().is_empty() {
        last
    } else {
        end.trim().parse::<usize>().ok()?.min(last)
    };
    (start <= end).then_some((start, end))
}

fn content_type(extension: &str) -> &'static str {
    match extension {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "webm" => "audio/webm",
        "m4a" | "mp4" => "audio/mp4",
        "ogg" | "opus" => "audio/ogg",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn plain(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    let mut response = Response::new(message.as_bytes().to_vec());
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_parsed_and_clamped() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=900-5000", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=500-100", 1000), None);
        assert_eq!(parse_range("bytes=0-10", 0), None);
        assert_eq!(parse_range("items=0-1", 10), None);
    }

    #[test]
    fn content_types_of_supported_media() {
        assert_eq!(content_type("webm"), "audio/webm");
        assert_eq!(content_type("jpg"), "image/jpeg");
        assert_eq!(content_type("exe"), "application/octet-stream");
    }
}
