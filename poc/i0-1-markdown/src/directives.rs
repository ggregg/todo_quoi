//! Repérage des blocs « maison » qui n'existent pas en Markdown standard :
//!
//! ```text
//! :::radio{label="Cible"}        groupe radio (un seul choix)
//! - ( ) Recette
//! - (x) Production
//! :::
//!
//! ::audio{src="../media/81bd.webm" duration=12.4}
//! ::file{src="../media/5c11.pdf" name="devis.pdf" size=48213}
//! ```
//!
//! Le corps est découpé en [`Segment`]s : du Markdown standard (confié à `pulldown-cmark`)
//! et des directives (analysées ici, à la main). Limite assumée : les directives ne sont
//! reconnues qu'au premier niveau (pas dans une liste ni une citation).

use crate::pm::Attrs;
use serde_json::{Number, Value};

/// Un morceau du corps de la note.
// [rust] Un `enum` peut porter des données différentes selon la variante
//        (≈ union discriminée TS : `{ kind: 'markdown', text } | { kind: 'radio', … }`).
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Markdown(String),
    Radio {
        attrs: Attrs,
        options: Vec<RadioOption>,
    },
    Leaf {
        name: String,
        attrs: Attrs,
        raw: String,
    },
}

/// Une option d'un groupe radio.
#[derive(Debug, Clone, PartialEq)]
pub struct RadioOption {
    pub checked: bool,
    pub label: String,
}

/// Découpe le corps en segments, en ignorant ce qui se trouve dans les blocs de code.
pub fn split_segments(body: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut markdown = String::new();
    let mut open_fence: Option<&str> = None;
    // [rust] `mut` : variable modifiable. Sans `mut`, toute variable est en lecture seule.
    let mut lines = body.lines();

    while let Some(line) = lines.next() {
        if let Some(fence) = open_fence {
            if line.trim_start().starts_with(fence) {
                open_fence = None;
            }
            push_line(&mut markdown, line);
            continue;
        }
        if let Some(fence) = fence_marker(line) {
            open_fence = Some(fence);
            push_line(&mut markdown, line);
            continue;
        }
        if let Some(rest) = line.strip_prefix(":::radio") {
            flush_markdown(&mut markdown, &mut segments);
            // [rust] `by_ref()` : on continue d'avancer dans le même itérateur `lines`.
            let options = lines
                .by_ref()
                .take_while(|inner| inner.trim() != ":::")
                .filter_map(parse_radio_option)
                .collect();
            segments.push(Segment::Radio {
                attrs: parse_attrs(rest),
                options,
            });
            continue;
        }
        if let Some((name, attrs)) = parse_leaf(line) {
            flush_markdown(&mut markdown, &mut segments);
            segments.push(Segment::Leaf {
                name,
                attrs,
                raw: line.to_string(),
            });
            continue;
        }
        push_line(&mut markdown, line);
    }
    flush_markdown(&mut markdown, &mut segments);
    segments
}

fn push_line(markdown: &mut String, line: &str) {
    markdown.push_str(line);
    markdown.push('\n');
}

fn flush_markdown(markdown: &mut String, segments: &mut Vec<Segment>) {
    if !markdown.trim().is_empty() {
        // [rust] `std::mem::take` : récupère le contenu et laisse une chaîne vide à la place.
        segments.push(Segment::Markdown(std::mem::take(markdown)));
    }
    markdown.clear();
}

/// Renvoie le marqueur d'ouverture d'un bloc de code (```` ``` ```` ou `~~~`), s'il y en a un.
fn fence_marker(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    ["```", "~~~"]
        .into_iter()
        .find(|marker| trimmed.starts_with(marker))
}

/// `- (x) Production` → option cochée « Production ».
fn parse_radio_option(line: &str) -> Option<RadioOption> {
    let rest = line.trim().strip_prefix("- (")?; // [rust] `?` sur un Option : renvoie None si absent.
    let (state, label) = rest.split_once(") ")?;
    Some(RadioOption {
        checked: state == "x",
        label: label.to_string(),
    })
}

/// `::audio{src="…" duration=12.4}` → `("audio", {src, duration})`.
fn parse_leaf(line: &str) -> Option<(String, Attrs)> {
    let trimmed = line.trim_end();
    if trimmed.starts_with(":::") || !trimmed.ends_with('}') {
        return None;
    }
    let rest = trimmed.strip_prefix("::")?;
    let brace = rest.find('{')?;
    let name = &rest[..brace];
    let is_valid_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric());
    is_valid_name.then(|| (name.to_string(), parse_attrs(&rest[brace..])))
}

/// Analyse `{clé="texte" nombre=12.4 drapeau=true}`.
///
/// Les valeurs entre guillemets sont des textes ; les autres sont converties en nombre ou
/// booléen si possible. Entrée mal formée → attributs vides (jamais d'erreur bloquante).
pub fn parse_attrs(source: &str) -> Attrs {
    let mut attrs = Attrs::new();
    let inner = source
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'));
    let Some(inner) = inner else {
        return attrs;
    };

    let mut chars = inner.chars().peekable();
    loop {
        // Sauter les espaces entre deux attributs.
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let key: String =
            std::iter::from_fn(|| chars.next_if(|c| *c != '=' && !c.is_whitespace())).collect();
        if key.is_empty() || chars.next() != Some('=') {
            return attrs;
        }
        let value = if chars.next_if_eq(&'"').is_some() {
            Value::String(read_quoted(&mut chars))
        } else {
            let raw: String =
                std::iter::from_fn(|| chars.next_if(|c| !c.is_whitespace())).collect();
            typed_value(&raw)
        };
        attrs.insert(key, value);
    }
}

/// Lit jusqu'au guillemet fermant, en gérant `\"` et `\\`.
fn read_quoted(chars: &mut impl Iterator<Item = char>) -> String {
    let mut value = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => value.extend(chars.next()),
            other => value.push(other),
        }
    }
    value
}

/// `true`/`false` → booléen, `480` → entier, `12.4` → décimal, sinon texte.
fn typed_value(raw: &str) -> Value {
    if let Ok(flag) = raw.parse::<bool>() {
        return Value::Bool(flag);
    }
    if let Ok(integer) = raw.parse::<i64>() {
        return Value::from(integer);
    }
    raw.parse::<f64>()
        .ok()
        .and_then(Number::from_f64)
        .map_or_else(|| Value::String(raw.to_string()), Value::Number)
}

/// Écrit des attributs : d'abord les clés de `order` (dans cet ordre), puis les autres
/// par ordre alphabétique. Les valeurs `null` sont omises.
pub fn render_attrs(attrs: Option<&Attrs>, order: &[&str]) -> String {
    let Some(attrs) = attrs else {
        return "{}".to_string();
    };
    let known = order.iter().filter_map(|key| attrs.get_key_value(*key));
    let others = attrs
        .iter()
        .filter(|(key, _)| !order.contains(&key.as_str()));
    let parts: Vec<String> = known
        .chain(others)
        .filter_map(|(key, value)| render_attr_value(value).map(|v| format!("{key}={v}")))
        .collect();
    format!("{{{}}}", parts.join(" "))
}

fn render_attr_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(format!(
            "\"{}\"",
            text.replace('\\', "\\\\").replace('"', "\\\"")
        )),
        Value::Bool(_) | Value::Number(_) => Some(value.to_string()),
        Value::Array(_) | Value::Object(_) => {
            Some(format!("\"{}\"", value.to_string().replace('"', "\\\"")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn attributes_are_typed() {
        let attrs = parse_attrs(r#"{src="../media/a b.webm" duration=12.4 size=480 muted=true}"#);
        assert_eq!(attrs["src"], json!("../media/a b.webm"));
        assert_eq!(attrs["duration"], json!(12.4));
        assert_eq!(attrs["size"], json!(480));
        assert_eq!(attrs["muted"], json!(true));
    }

    #[test]
    fn attributes_round_trip_in_requested_order() {
        let attrs = parse_attrs(r#"{name="devis \"final\".pdf" src="x.pdf" size=12}"#);
        let rendered = render_attrs(Some(&attrs), &["src", "name", "size"]);
        assert_eq!(
            rendered,
            r#"{src="x.pdf" name="devis \"final\".pdf" size=12}"#
        );
    }

    #[test]
    fn directives_inside_code_blocks_are_ignored() {
        let body = "```\n::audio{src=\"x\"}\n```\n";
        assert_eq!(
            split_segments(body),
            vec![Segment::Markdown(body.to_string())]
        );
    }

    #[test]
    fn radio_group_is_parsed() {
        let segments = split_segments(":::radio{label=\"Cible\"}\n- ( ) A\n- (x) B\n:::\n");
        let Segment::Radio { attrs, options } = &segments[0] else {
            panic!("groupe radio attendu : {segments:?}");
        };
        assert_eq!(attrs["label"], json!("Cible"));
        assert_eq!(options.len(), 2);
        assert!(options[1].checked);
    }
}
