//! Format d'un fichier de tâche : texte ↔ [`Task`]. Aucun accès disque ici.
//!
//! ```text
//! +++
//! schema_version = 1
//! id = "0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b1e"
//! order = "a0V"
//! title = "Préparer la release 1.2"
//! done = false
//! tags = ["release"]
//! created_at = 2026-11-05T16:02:11.120Z
//! updated_at = 2026-11-05T17:25:12.345Z
//! updated_by = "mac-greg"
//! +++
//! Corps de la note en Markdown…
//! ```
//!
//! La table facultative `[view]` porte la présentation de la liste des sous-tâches (format :
//! [`crate::view_format`]). Une vue illisible est traitée comme un champ inconnu : conservée.
//!
//! **Tâche chiffrée** (ADR-0010) : même forme, l'en-tête reste **entièrement en clair** (titre,
//! étiquettes, vue, statut…) ; seul le **corps** — la note — est remplacé par son texte chiffré.
//! `schema_version = 2` : une version qui ne connaît pas le chiffrement écarte le fichier au
//! lieu de l'écraser.
//! ```text
//! +++
//! schema_version = 2
//! id = "…"
//! order = "a0V"
//! title = "Préparer la release 1.2"
//! done = false
//! encrypted = true
//! key_id = "0123456789abcdef0123456789abcdef"
//! …
//! +++
//! xc20p1:5f0c…
//! ```
//!
//! Les champs d'en-tête **inconnus** (ajoutés par une version plus récente, ou vue illisible)
//! sont rendus à l'appelant pour être réécrits tels quels (ADR-0004).

use crate::view_format::ViewFile;
use serde::{Deserialize, Serialize};
use toml::value::Datetime;
use tq_domain::view::ListView;
use tq_domain::{
    Confidentiality, DeviceId, KeyId, NoteBody, OrderKey, SealedContent, Tag, Task, TaskId,
    TaskRecord, TaskStatus, Timestamp, Title,
};

/// Version de format la plus récente que cette application sait lire.
pub const SCHEMA_VERSION: u32 = 2;
/// Version écrite pour une tâche en clair et pour le manifeste (inchangée depuis I1).
pub const PLAIN_SCHEMA_VERSION: u32 = 1;
/// Version écrite pour une tâche chiffrée (I6).
const SEALED_SCHEMA_VERSION: u32 = 2;
/// Longueur des lignes du texte chiffré dans le fichier (lisibilité, diffs).
const SEALED_LINE_LENGTH: usize = 76;

const OPENING: &str = "+++\n";
const CLOSING: &str = "\n+++\n";

/// Fichier de tâche mal formé.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    /// Pas d'en-tête `+++ … +++` en début de fichier.
    #[error("en-tête +++ absent ou non refermé")]
    MissingHeader,
    /// En-tête TOML illisible.
    #[error("en-tête TOML invalide : {0}")]
    Toml(String),
    /// Un champ ne respecte pas les règles du domaine.
    #[error("champ « {field} » invalide : {reason}")]
    InvalidField {
        /// Nom du champ.
        field: &'static str,
        /// Explication.
        reason: String,
    },
    /// Tâche marquée « chiffrée » mais pas encore scellée : le stockage doit la chiffrer avant de
    /// l'écrire (jamais de contenu chiffré écrit en clair).
    #[error("tâche chiffrée non scellée : écriture refusée")]
    NotSealed,
    /// Écrit par une version plus récente de l'application.
    #[error("format version {0}, plus récent que celui de cette application ({SCHEMA_VERSION})")]
    NewerSchema(u32),
}

/// Contenu d'un fichier relu.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTaskFile {
    /// La tâche.
    pub task: Task,
    /// Champs d'en-tête non gérés par cette version, à conserver à la réécriture.
    pub unknown: toml::Table,
}

/// En-tête tel qu'il est écrit dans le fichier (types « fichier » : textes, dates TOML).
#[derive(Debug, Serialize, Deserialize)]
struct Header {
    schema_version: u32,
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
    order: String,
    title: String,
    done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    done_at: Option<Datetime>,
    #[serde(default)]
    tags: Vec<String>,
    created_at: Datetime,
    updated_at: Datetime,
    updated_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deleted_at: Option<Datetime>,
    #[serde(default)]
    encrypted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_id: Option<String>,
    #[serde(flatten)]
    unknown: toml::Table,
}

/// Lit un fichier de tâche. Les fins de ligne Windows (`\r\n`) sont acceptées.
///
/// # Erreurs
/// [`FormatError`] si l'en-tête est absent, illisible ou contient une valeur invalide.
pub fn parse_task_file(text: &str) -> Result<ParsedTaskFile, FormatError> {
    let text = text.replace("\r\n", "\n");
    let (header_text, body) = split(&text).ok_or(FormatError::MissingHeader)?;
    let header: Header =
        toml::from_str(header_text).map_err(|e| FormatError::Toml(e.message().to_string()))?;
    if header.schema_version > SCHEMA_VERSION {
        return Err(FormatError::NewerSchema(header.schema_version));
    }
    let mut unknown = header.unknown.clone();
    let view = take_view(&mut unknown);
    let mut record = record_from(header, body)?;
    record.view = view;
    let task = Task::from_record(record);
    Ok(ParsedTaskFile { task, unknown })
}

/// Écrit un fichier de tâche, en reprenant les champs inconnus `unknown`.
///
/// # Erreurs
/// [`FormatError::Toml`] si la sérialisation échoue (ne devrait pas arriver).
/// Une tâche scellée est écrite avec sa note chiffrée à la place de la note.
///
/// # Erreurs
/// [`FormatError::NotSealed`] pour une tâche « chiffrée » pas encore scellée ;
/// [`FormatError::Toml`] si la sérialisation échoue (ne devrait pas arriver).
pub fn render_task_file(task: &Task, unknown: &toml::Table) -> Result<String, FormatError> {
    let record = task.to_record();
    let (header, body) = match &record.confidentiality {
        Confidentiality::Plain => (
            header_from(&record, unknown.clone())?,
            record.note.as_markdown().to_string(),
        ),
        Confidentiality::Sealed(content) => (
            sealed_header_from(&record, content, unknown.clone())?,
            wrap_lines(&content.sealed),
        ),
        Confidentiality::Encrypted => return Err(FormatError::NotSealed),
    };
    let header_text = toml::to_string(&header).map_err(|e| FormatError::Toml(e.to_string()))?;
    Ok(format!("{OPENING}{header_text}+++\n{body}"))
}

/// Texte à chiffrer pour une tâche « chiffrée » : sa note (Markdown).
pub fn plain_text_to_seal(task: &Task) -> String {
    task.note().as_markdown().to_string()
}

/// Tâche lisible à partir d'une tâche scellée relue (`outer`, tout en clair sauf la note) et de
/// sa note déchiffrée (`note`).
pub fn unsealed(outer: &Task, note: &str) -> Task {
    let mut record = outer.to_record();
    record.note = NoteBody::from_markdown(note);
    record.confidentiality = Confidentiality::Encrypted;
    Task::from_record(record)
}

/// Données associées au chiffrement d'une tâche : le texte chiffré est lié à **sa** tâche.
pub fn sealing_context(id: &TaskId) -> Vec<u8> {
    format!("todoquoi-task:{id}").into_bytes()
}

/// Sépare l'en-tête (sans les `+++`) du corps.
fn split(text: &str) -> Option<(&str, &str)> {
    let after_opening = text.strip_prefix(OPENING)?;
    let end = after_opening.find(CLOSING)?;
    Some((
        &after_opening[..=end],
        &after_opening[end + CLOSING.len()..],
    ))
}

// ----- Conversions fichier ↔ domaine ----------------------------------------------------------

fn record_from(header: Header, body: &str) -> Result<TaskRecord, FormatError> {
    let confidentiality = confidentiality_from(&header, body)?;
    let note = if confidentiality.is_sealed() {
        NoteBody::empty() // le corps est le texte chiffré, pas une note
    } else {
        NoteBody::from_markdown(body)
    };
    let updated_at = timestamp("updated_at", &header.updated_at)?;
    let status = status_from(&header, updated_at)?;
    Ok(TaskRecord {
        id: field("id", TaskId::parse(&header.id))?,
        parent: header
            .parent
            .as_deref()
            .map(|p| field("parent", TaskId::parse(p)))
            .transpose()?,
        order: field("order", OrderKey::parse(&header.order))?,
        title: field("title", Title::parse(&header.title))?,
        status,
        tags: header
            .tags
            .iter()
            .map(|t| field("tags", Tag::parse(t)))
            .collect::<Result<_, _>>()?,
        note,
        created_at: timestamp("created_at", &header.created_at)?,
        updated_at,
        updated_by: field("updated_by", DeviceId::parse(&header.updated_by))?,
        deleted_at: header
            .deleted_at
            .as_ref()
            .map(|d| timestamp("deleted_at", d))
            .transpose()?,
        confidentiality,
        view: None,
    })
}

/// Terminée sans date de clôture (fichier écrit à la main) : on prend la date de modification.
fn status_from(header: &Header, updated_at: Timestamp) -> Result<TaskStatus, FormatError> {
    if !header.done {
        return Ok(TaskStatus::Open);
    }
    let at = header
        .done_at
        .as_ref()
        .map(|d| timestamp("done_at", d))
        .transpose()?;
    Ok(TaskStatus::Done {
        at: at.unwrap_or(updated_at),
    })
}

/// En clair, ou scellée (le corps est alors le texte chiffré). Jamais « chiffrée » à la
/// lecture : c'est le stockage qui la déchiffre, s'il en a la clé.
fn confidentiality_from(header: &Header, body: &str) -> Result<Confidentiality, FormatError> {
    if !header.encrypted {
        return Ok(Confidentiality::Plain);
    }
    let key_id = header.key_id.as_deref().ok_or(FormatError::InvalidField {
        field: "key_id",
        reason: "absent d'une tâche chiffrée".into(),
    })?;
    Ok(Confidentiality::Sealed(SealedContent {
        key_id: field("key_id", KeyId::parse(key_id))?,
        sealed: body.split_whitespace().collect(),
    }))
}

/// En-tête d'une tâche scellée : tout en clair, plus l'identifiant du coffre.
fn sealed_header_from(
    record: &TaskRecord,
    content: &SealedContent,
    unknown: toml::Table,
) -> Result<Header, FormatError> {
    let mut header = header_from(record, unknown)?;
    header.schema_version = SEALED_SCHEMA_VERSION;
    header.key_id = Some(content.key_id.to_string());
    Ok(header)
}

/// Coupe le texte chiffré en lignes de [`SEALED_LINE_LENGTH`] caractères.
fn wrap_lines(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    characters
        .chunks(SEALED_LINE_LENGTH)
        .map(|line| line.iter().collect::<String>() + "\n")
        .collect()
}

/// Retire `[view]` des champs inconnus s'il est lisible ; sinon l'y laisse (conservé tel quel).
fn take_view(unknown: &mut toml::Table) -> Option<ListView> {
    let raw = unknown.get(VIEW_KEY)?.clone();
    let view = raw.try_into::<ViewFile>().ok()?.to_view()?;
    unknown.remove(VIEW_KEY);
    Some(view)
}

/// Nom de la table de la vue dans l'en-tête.
const VIEW_KEY: &str = "view";

fn header_from(record: &TaskRecord, mut unknown: toml::Table) -> Result<Header, FormatError> {
    if let Some(view) = &record.view {
        let value = toml::Value::try_from(ViewFile::from_view(view))
            .map_err(|e| FormatError::Toml(e.to_string()))?;
        unknown.insert(VIEW_KEY.to_string(), value);
    }
    let done_at = match record.status {
        TaskStatus::Open => None,
        TaskStatus::Done { at } => Some(datetime(at)?),
    };
    Ok(Header {
        schema_version: PLAIN_SCHEMA_VERSION,
        id: record.id.to_string(),
        parent: record.parent.as_ref().map(TaskId::to_string),
        order: record.order.to_string(),
        title: record.title.to_string(),
        done: done_at.is_some(),
        done_at,
        tags: record.tags.iter().map(Tag::to_string).collect(),
        created_at: datetime(record.created_at)?,
        updated_at: datetime(record.updated_at)?,
        updated_by: record.updated_by.to_string(),
        deleted_at: record.deleted_at.map(datetime).transpose()?,
        encrypted: record.confidentiality.is_encrypted(),
        key_id: None,
        unknown,
    })
}

/// Attache le nom du champ à une erreur du domaine.
fn field<T>(
    name: &'static str,
    result: Result<T, tq_domain::DomainError>,
) -> Result<T, FormatError> {
    result.map_err(|error| FormatError::InvalidField {
        field: name,
        reason: error.to_string(),
    })
}

/// Date TOML → `Timestamp`. Les dates sans fuseau (`2026-11-05T16:02:11`) sont refusées.
fn timestamp(name: &'static str, value: &Datetime) -> Result<Timestamp, FormatError> {
    field(name, Timestamp::parse_rfc3339(&value.to_string()))
}

/// `Timestamp` → date TOML (même texte RFC 3339).
fn datetime(value: Timestamp) -> Result<Datetime, FormatError> {
    value
        .to_rfc3339()
        .parse::<Datetime>()
        .map_err(|error| FormatError::Toml(error.to_string()))
}
