//! Corps de la note d'une tâche.

/// Texte de la note, en Markdown étendu (directives `:::radio`, `::audio`, `::file`).
///
/// **I1** : la note est conservée telle quelle (texte brut). Le modèle structuré en blocs
/// (`Note { blocks: Vec<Block> }`, architecture §2.1) arrive en I2 avec l'éditeur ; il réutilisera
/// le convertisseur validé au POC I0-1.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteBody(String);

impl NoteBody {
    /// Note vide.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Crée une note à partir de son texte Markdown (aucune validation : tout texte est accepté).
    pub fn from_markdown(markdown: impl Into<String>) -> Self {
        Self(markdown.into())
    }

    /// Le texte Markdown.
    pub fn as_markdown(&self) -> &str {
        &self.0
    }

    /// Vrai si la note ne contient que des espaces.
    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }
}
