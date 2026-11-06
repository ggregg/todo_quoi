//! Le format JSON de TipTap (ProseMirror), tel que renvoyé par `editor.getJSON()`.
//!
//! Exemple : `{ "type": "paragraph", "content": [{ "type": "text", "text": "Bonjour",
//! "marks": [{ "type": "bold" }] }] }`.
//!
//! Le modèle est volontairement **générique** (le type du nœud est une chaîne) : c'est un
//! format d'échange, pas un modèle métier. Le domaine aura un `enum Block` typé.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Attributs d'un nœud ou d'une marque (`{ "level": 2 }`, `{ "href": "…" }`…).
pub type Attrs = Map<String, Value>;

/// Un nœud ProseMirror : bloc (`paragraph`, `heading`…) ou texte (`text`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmNode {
    // [rust] `type` est un mot réservé en Rust : on nomme le champ `kind`
    //        et on demande à serde de l'appeler `type` en JSON.
    /// Type du nœud (`doc`, `paragraph`, `text`, `taskItem`…).
    #[serde(rename = "type")]
    pub kind: String,
    // [rust] `Option<T>` = valeur facultative (≈ `T | undefined`). Les attributs serde
    //        disent : absent en JSON → `None`, et `None` → champ omis à l'écriture.
    /// Attributs (`{ "level": 2 }`, `{ "checked": true }`…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
    /// Nœuds enfants (blocs ou contenu en ligne).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<PmNode>>,
    /// Texte, pour un nœud `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Mise en forme, pour un nœud `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marks: Option<Vec<PmMark>>,
}

/// Une marque appliquée à du texte : `bold`, `italic`, `strike`, `code`, `link`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmMark {
    /// Type de la marque.
    #[serde(rename = "type")]
    pub kind: String,
    /// Attributs (ex. `href` d'un lien).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Attrs>,
}

// [rust] `impl Type { … }` regroupe les méthodes d'un type (≈ le corps d'une classe TS).
impl PmNode {
    /// Crée un nœud vide d'un type donné (`PmNode::block("paragraph")`).
    pub fn block(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            attrs: None,
            content: None,
            text: None,
            marks: None,
        }
    }

    /// Crée un nœud texte portant les marques données.
    pub fn text(text: &str, marks: &[PmMark]) -> Self {
        Self {
            text: Some(text.to_string()),
            marks: if marks.is_empty() {
                None
            } else {
                Some(marks.to_vec())
            },
            ..Self::block("text") // [rust] « reprendre les autres champs de cette valeur » (≈ spread `...`).
        }
    }

    /// Ajoute un attribut et renvoie le nœud (style « builder »).
    // [rust] `mut self` : la méthode prend possession du nœud, le modifie et le rend.
    pub fn with_attr(mut self, key: &str, value: Value) -> Self {
        self.set_attr(key, value);
        self
    }

    /// Ajoute ou remplace un attribut.
    pub fn set_attr(&mut self, key: &str, value: Value) {
        self.attrs
            .get_or_insert_with(Attrs::new)
            .insert(key.to_string(), value);
    }

    /// Lit un attribut, en traitant `null` comme absent (TipTap écrit souvent `null`).
    pub fn attr(&self, key: &str) -> Option<&Value> {
        self.attrs
            .as_ref()
            .and_then(|attrs| attrs.get(key))
            .filter(|value| !value.is_null())
    }

    /// Lit un attribut texte.
    pub fn attr_str(&self, key: &str) -> Option<&str> {
        self.attr(key).and_then(Value::as_str)
    }

    /// Lit un attribut entier positif (accepte aussi `3.0`, que JavaScript peut produire).
    pub fn attr_u64(&self, key: &str) -> Option<u64> {
        let value = self.attr(key)?;
        value.as_u64().or_else(|| value.as_f64().map(|n| n as u64))
    }

    /// Lit un attribut booléen.
    pub fn attr_bool(&self, key: &str) -> bool {
        self.attr(key).and_then(Value::as_bool).unwrap_or(false)
    }

    /// Les enfants du nœud (liste vide s'il n'en a pas).
    pub fn children(&self) -> &[PmNode] {
        self.content.as_deref().unwrap_or(&[])
    }

    /// Ajoute un enfant à la fin.
    pub fn push_child(&mut self, child: PmNode) {
        self.content.get_or_insert_with(Vec::new).push(child);
    }

    /// Les marques du nœud texte (liste vide s'il n'en a pas).
    pub fn marks(&self) -> &[PmMark] {
        self.marks.as_deref().unwrap_or(&[])
    }

    /// Concatène le texte de tous les nœuds texte descendants (sans mise en forme).
    pub fn plain_text(&self) -> String {
        match &self.text {
            Some(text) => text.clone(),
            None => self.children().iter().map(PmNode::plain_text).collect(),
        }
    }
}

impl PmMark {
    /// Crée une marque sans attribut (`PmMark::new("bold")`).
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            attrs: None,
        }
    }

    /// Lit un attribut texte de la marque (ex. `href` d'un lien), `null` = absent.
    pub fn attr_str(&self, key: &str) -> Option<&str> {
        self.attrs
            .as_ref()
            .and_then(|attrs| attrs.get(key))
            .and_then(Value::as_str)
    }
}
