//! Fabriquer des tâches de test lisiblement :
//! `TaskBuilder::new(1, "Courses").parent(2).done().build()`.

use tq_domain::{
    BaseId, Confidentiality, DeviceId, KeyId, NoteBody, OrderKey, SealedContent, Tag, Task, TaskId,
    TaskRecord, TaskStatus, Timestamp, Title,
};

/// Identifiant de test n° `number` : `00000000-0000-7000-8000-00000000000a` pour 10.
pub fn task_id(number: u64) -> TaskId {
    match TaskId::parse(&format!("00000000-0000-7000-8000-{number:012x}")) {
        Ok(id) => id,
        // Impossible : le format ci-dessus est toujours un UUID valide.
        Err(error) => panic!("identifiant de test invalide : {error}"),
    }
}

/// Constructeur de tâche de test : valeurs par défaut raisonnables, à surcharger au besoin.
pub struct TaskBuilder {
    record: TaskRecord,
    base: BaseId,
}

impl TaskBuilder {
    /// Tâche ouverte n° `number`, au premier niveau.
    pub fn new(number: u64, title: &str) -> Self {
        let at = Timestamp::from_millis(1_000);
        Self {
            record: TaskRecord {
                id: task_id(number),
                parent: None,
                order: OrderKey::first(),
                title: valid(Title::parse(title)),
                status: TaskStatus::Open,
                tags: Vec::new(),
                note: NoteBody::empty(),
                created_at: at,
                updated_at: at,
                updated_by: valid(DeviceId::parse("test")),
                deleted_at: None,
                confidentiality: Confidentiality::Plain,
                view: None,
            },
            base: BaseId::main(),
        }
    }

    /// Dans la base `name` (ADR-0009).
    pub fn in_base(mut self, name: &str) -> Self {
        self.base = valid(BaseId::parse(name));
        self
    }

    /// Sous-tâche de la tâche n° `number`.
    pub fn parent(mut self, number: u64) -> Self {
        self.record.parent = Some(task_id(number));
        self
    }

    /// Position (clé d'ordre) explicite.
    pub fn order(mut self, key: &str) -> Self {
        self.record.order = valid(OrderKey::parse(key));
        self
    }

    /// Terminée.
    pub fn done(mut self) -> Self {
        self.record.status = TaskStatus::Done {
            at: Timestamp::from_millis(2_000),
        };
        self
    }

    /// Dans la corbeille depuis `millis`.
    pub fn deleted_at(mut self, millis: i64) -> Self {
        self.record.deleted_at = Some(Timestamp::from_millis(millis));
        self
    }

    /// Avec ces étiquettes.
    pub fn tags(mut self, tags: &[&str]) -> Self {
        self.record.tags = tags.iter().map(|tag| valid(Tag::parse(tag))).collect();
        self
    }

    /// Avec cette note.
    pub fn note(mut self, markdown: &str) -> Self {
        self.record.note = NoteBody::from_markdown(markdown);
        self
    }

    /// Chiffrée, coffre ouvert (contenu lisible).
    pub fn encrypted(mut self) -> Self {
        self.record.confidentiality = Confidentiality::Encrypted;
        self
    }

    /// Chiffrée, coffre verrouillé : contenu illisible (texte chiffré factice).
    pub fn sealed(mut self) -> Self {
        self.record.confidentiality = Confidentiality::Sealed(SealedContent {
            key_id: valid(KeyId::parse("0123456789abcdef0123456789abcdef")),
            sealed: "fake:00::".to_string(),
        });
        self
    }

    /// Construit la tâche.
    pub fn build(self) -> Task {
        Task::from_record(self.record).in_base(self.base)
    }
}

/// Une valeur de test invalide est une erreur dans le test lui-même : on s'arrête net.
fn valid<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("donnée de test invalide : {error}"),
    }
}
