//! Markdown étendu → document TipTap.
//!
//! `pulldown-cmark` ne construit pas d'arbre : il émet un **flux d'événements**
//! (`Start(Paragraph)`, `Text("Bonjour")`, `End(Paragraph)`…), un peu comme un parseur SAX.
//! Le [`Builder`] reconstruit l'arbre ProseMirror avec une **pile** de nœuds ouverts.
//!
//! Deux adaptations au schéma TipTap :
//! - un élément de liste ou une cellule de tableau doit contenir un paragraphe, alors que
//!   le Markdown « serré » n'en émet pas : on ouvre alors un paragraphe **implicite** ;
//! - une image est un bloc dans TipTap, alors qu'elle est en ligne en Markdown : on la sort
//!   de son paragraphe.

use crate::directives::{RadioOption, Segment, split_segments};
use crate::pm::{Attrs, PmMark, PmNode};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde_json::{Value, json};

/// Convertit le corps Markdown d'une note en document TipTap (`{ "type": "doc", … }`).
pub fn markdown_to_doc(body: &str) -> PmNode {
    let mut doc = PmNode::block("doc");
    for segment in split_segments(body) {
        match segment {
            Segment::Markdown(text) => {
                for block in parse_standard_markdown(&text) {
                    doc.push_child(block);
                }
            }
            Segment::Radio { attrs, options } => doc.push_child(radio_group(attrs, options)),
            Segment::Leaf { name, attrs, raw } => doc.push_child(leaf_block(&name, attrs, &raw)),
        }
    }
    // Le schéma TipTap exige au moins un bloc dans un document.
    if doc.children().is_empty() {
        doc.push_child(PmNode::block("paragraph"));
    }
    doc
}

fn radio_group(attrs: Attrs, options: Vec<RadioOption>) -> PmNode {
    let mut group = PmNode::block("radioGroup");
    group.attrs = Some(attrs);
    for option in options {
        let mut node = PmNode::block("radioOption").with_attr("checked", json!(option.checked));
        if !option.label.is_empty() {
            node.push_child(PmNode::text(&option.label, &[]));
        }
        group.push_child(node);
    }
    group
}

fn leaf_block(name: &str, attrs: Attrs, raw: &str) -> PmNode {
    let kind = match name {
        "audio" => "audio",
        "file" => "attachment",
        "filelink" => "fileLink",
        // Directive inconnue : on la garde comme texte, rien n'est perdu.
        _ => return paragraph_with_text(raw),
    };
    let mut node = PmNode::block(kind);
    node.attrs = Some(attrs);
    node
}

fn paragraph_with_text(text: &str) -> PmNode {
    let mut paragraph = PmNode::block("paragraph");
    paragraph.push_child(PmNode::text(text, &[]));
    paragraph
}

/// Analyse un morceau de Markdown standard (GFM : tableaux, cases à cocher, barré).
fn parse_standard_markdown(text: &str) -> Vec<PmNode> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut builder = Builder::new();
    for event in Parser::new_ext(text, options) {
        builder.handle(event);
    }
    builder.finish()
}

/// Un nœud en cours de construction sur la pile.
struct Frame {
    node: PmNode,
    /// Paragraphe ajouté par nous (Markdown serré) : on le ferme dès qu'un bloc commence.
    implicit: bool,
}

/// Image en cours de lecture : son texte alternatif arrive en événements `Text` séparés.
struct PendingImage {
    src: String,
    title: String,
    alt: String,
}

/// Reconstruit l'arbre à partir du flux d'événements de `pulldown-cmark`.
struct Builder {
    /// Pile des nœuds ouverts. Invariant : `stack[0]` est la racine, jamais retirée.
    stack: Vec<Frame>,
    /// Marques actives (gras, italique…) appliquées au texte lu.
    marks: Vec<PmMark>,
    image: Option<PendingImage>,
    in_table_head: bool,
    /// Vrai juste après une image : un `{width=480}` qui suit lui appartient.
    after_image: bool,
}

impl Builder {
    fn new() -> Self {
        Self {
            stack: vec![Frame {
                node: PmNode::block("doc"),
                implicit: false,
            }],
            marks: Vec::new(),
            image: None,
            in_table_head: false,
            after_image: false,
        }
    }

    /// Ferme tout ce qui reste ouvert et renvoie les blocs de premier niveau.
    fn finish(mut self) -> Vec<PmNode> {
        while self.stack.len() > 1 {
            self.pop_and_attach();
        }
        // [rust] `.remove(0)` retire la racine ; `.content.unwrap_or_default()` : ses enfants ou `[]`.
        self.stack.remove(0).node.content.unwrap_or_default()
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self.inline_code(&code),
            Event::SoftBreak => self.text(" "),
            Event::HardBreak => self.push_inline(PmNode::block("hardBreak")),
            Event::Rule => {
                self.close_implicit_paragraph();
                self.top().push_child(PmNode::block("horizontalRule"));
            }
            Event::TaskListMarker(checked) => self.mark_task_item(checked),
            Event::Html(html) | Event::InlineHtml(html) => self.text(&html),
            // [rust] `_` = « tous les autres cas » (notes de bas de page, maths…) : ignorés ici.
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Emphasis
            | Tag::Strong
            | Tag::Strikethrough
            | Tag::Link { .. }
            | Tag::Image { .. } => {
                self.start_inline(tag);
            }
            _ => self.start_block(tag),
        }
    }

    /// Début d'une mise en forme ou d'une image (contenu en ligne).
    fn start_inline(&mut self, tag: Tag) {
        match tag {
            Tag::Emphasis => self.marks.push(PmMark::new("italic")),
            Tag::Strong => self.marks.push(PmMark::new("bold")),
            Tag::Strikethrough => self.marks.push(PmMark::new("strike")),
            Tag::Link {
                dest_url, title, ..
            } => self.marks.push(link_mark(&dest_url, &title)),
            Tag::Image {
                dest_url, title, ..
            } => {
                self.image = Some(PendingImage {
                    src: dest_url.to_string(),
                    title: title.to_string(),
                    alt: String::new(),
                });
            }
            _ => {}
        }
    }

    /// Début d'un bloc (paragraphe, titre, liste, tableau…).
    fn start_block(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => self.open(PmNode::block("paragraph")),
            Tag::Heading { level, .. } => {
                self.open(PmNode::block("heading").with_attr("level", json!(level as u8)))
            }
            Tag::BlockQuote(_) => self.open(PmNode::block("blockquote")),
            Tag::CodeBlock(kind) => {
                self.open(PmNode::block("codeBlock").with_attr("language", language(kind)))
            }
            Tag::List(Some(start)) => {
                self.open(PmNode::block("orderedList").with_attr("start", json!(start)))
            }
            Tag::List(None) => self.open(PmNode::block("bulletList")),
            Tag::Item => self.open(PmNode::block("listItem")),
            Tag::Table(_) => self.open(PmNode::block("table")),
            Tag::TableHead => {
                self.in_table_head = true;
                self.open(PmNode::block("tableRow"));
            }
            Tag::TableRow => self.open(PmNode::block("tableRow")),
            Tag::TableCell => {
                let kind = if self.in_table_head {
                    "tableHeader"
                } else {
                    "tableCell"
                };
                self.open(PmNode::block(kind));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Table
            | TagEnd::TableRow
            | TagEnd::TableCell => self.close(),
            TagEnd::TableHead => {
                self.close();
                self.in_table_head = false;
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                self.marks.pop();
            }
            TagEnd::Image => self.finish_image(),
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if let Some(image) = &mut self.image {
            image.alt.push_str(text);
            return;
        }
        if self.top().kind == "codeBlock" {
            append_code(self.top(), text);
            return;
        }
        let remaining = if self.after_image {
            self.take_image_attrs(text)
        } else {
            text
        };
        if !remaining.is_empty() {
            self.push_inline(PmNode::text(remaining, &self.marks.clone()));
        }
    }

    fn inline_code(&mut self, code: &str) {
        let mut marks = self.marks.clone();
        marks.push(PmMark::new("code"));
        self.push_inline(PmNode::text(code, &marks));
    }

    fn finish_image(&mut self) {
        // [rust] `Option::take` : récupère la valeur et laisse `None` à la place.
        let Some(image) = self.image.take() else {
            return;
        };
        let mut node = PmNode::block("image").with_attr("src", json!(image.src));
        if !image.alt.is_empty() {
            node.set_attr("alt", json!(image.alt));
        }
        if !image.title.is_empty() {
            node.set_attr("title", json!(image.title));
        }
        self.push_inline(node);
        self.after_image = true;
    }

    /// Si le texte commence par `{…}`, ce sont des attributs de l'image précédente.
    /// Renvoie le texte restant.
    // [rust] La durée de vie `'t` dit : « le texte renvoyé est un morceau du texte reçu ».
    //        C'est la seule annotation de ce genre du POC ; le compilateur l'exige ici.
    fn take_image_attrs<'t>(&mut self, text: &'t str) -> &'t str {
        self.after_image = false;
        let Some(end) = text.strip_prefix('{').and_then(|_| text.find('}')) else {
            return text;
        };
        let attrs = crate::directives::parse_attrs(&text[..=end]);
        if let Some(image) = self.top().content.as_mut().and_then(|c| c.last_mut()) {
            for (key, value) in attrs {
                image.set_attr(&key, value);
            }
        }
        &text[end + 1..]
    }

    /// `- [x]` : l'élément de liste en cours devient une case à cocher.
    fn mark_task_item(&mut self, checked: bool) {
        let item = self
            .stack
            .iter_mut()
            .rev()
            .find(|frame| frame.node.kind == "listItem");
        if let Some(frame) = item {
            frame.node.kind = "taskItem".to_string();
            frame.node.set_attr("checked", json!(checked));
        }
    }

    // ----- Gestion de la pile ---------------------------------------------------------

    /// Le nœud ouvert le plus profond.
    // [rust] `#[expect(…)]` : on autorise ici, et seulement ici, un lint interdit ailleurs, en
    //        disant pourquoi. Si un jour le lint ne se déclenche plus, le compilateur le signale.
    #[expect(
        clippy::expect_used,
        reason = "invariant : la racine n'est jamais retirée de la pile"
    )]
    fn top(&mut self) -> &mut PmNode {
        &mut self
            .stack
            .last_mut()
            .expect("la racine est toujours présente")
            .node
    }

    fn open(&mut self, node: PmNode) {
        self.close_implicit_paragraph();
        self.stack.push(Frame {
            node,
            implicit: false,
        });
    }

    fn close(&mut self) {
        self.close_implicit_paragraph();
        self.pop_and_attach();
    }

    fn close_implicit_paragraph(&mut self) {
        if self.stack.last().is_some_and(|frame| frame.implicit) {
            self.pop_and_attach();
        }
    }

    /// Retire le nœud du sommet de la pile, le finalise et l'attache à son parent.
    fn pop_and_attach(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        if let Some(frame) = self.stack.pop() {
            for node in finalize(frame.node) {
                self.top().push_child(node);
            }
        }
    }

    /// Ajoute un contenu en ligne (texte, image, saut de ligne) au nœud courant,
    /// en ouvrant un paragraphe implicite si le conteneur l'exige.
    fn push_inline(&mut self, node: PmNode) {
        self.after_image = false;
        if needs_paragraph(&self.top().kind) {
            self.stack.push(Frame {
                node: PmNode::block("paragraph"),
                implicit: true,
            });
        }
        append_inline(self.top(), node);
    }
}

/// Conteneurs dont le contenu doit être enveloppé dans un paragraphe (schéma TipTap).
fn needs_paragraph(kind: &str) -> bool {
    matches!(
        kind,
        "doc" | "listItem" | "taskItem" | "tableCell" | "tableHeader" | "blockquote"
    )
}

/// Ajoute un nœud en ligne ; fusionne deux textes consécutifs de mêmes marques.
fn append_inline(parent: &mut PmNode, node: PmNode) {
    // [rust] Chaîne de conditions `if let … && … && let …` : chaque `let` doit réussir
    //        (≈ une suite de gardes `if (!x) return;` en TS, mais en une seule expression).
    let last = parent.content.as_mut().and_then(|c| c.last_mut());
    if let (Some(text), Some(last)) = (&node.text, last)
        && last.kind == "text"
        && last.marks == node.marks
        && let Some(last_text) = &mut last.text
    {
        last_text.push_str(text);
        return;
    }
    parent.push_child(node);
}

fn append_code(code_block: &mut PmNode, text: &str) {
    match code_block
        .content
        .as_mut()
        .and_then(|c| c.first_mut())
        .and_then(|n| n.text.as_mut())
    {
        Some(existing) => existing.push_str(text),
        None => code_block.push_child(PmNode::text(text, &[])),
    }
}

fn language(kind: CodeBlockKind) -> Value {
    match kind {
        CodeBlockKind::Fenced(lang) if !lang.is_empty() => json!(lang.to_string()),
        _ => Value::Null,
    }
}

fn link_mark(href: &str, title: &str) -> PmMark {
    let mut attrs = Attrs::new();
    attrs.insert("href".to_string(), json!(href));
    if !title.is_empty() {
        attrs.insert("title".to_string(), json!(title));
    }
    PmMark {
        kind: "link".to_string(),
        attrs: Some(attrs),
    }
}

/// Ajustements au schéma TipTap au moment où un nœud est fermé.
/// Renvoie une liste car un paragraphe contenant des images est éclaté en plusieurs blocs.
fn finalize(mut node: PmNode) -> Vec<PmNode> {
    match node.kind.as_str() {
        "paragraph" => return hoist_images(node),
        "codeBlock" => trim_final_newline(&mut node),
        "bulletList" => {
            let all_tasks = node.children().iter().all(|item| item.kind == "taskItem");
            if all_tasks && !node.children().is_empty() {
                node.kind = "taskList".to_string();
            }
        }
        // [rust] `if` après un motif = « garde » : la branche ne s'applique que si la condition est vraie.
        "listItem" | "taskItem" | "tableCell" | "tableHeader" | "blockquote"
            if node.children().is_empty() =>
        {
            node.push_child(PmNode::block("paragraph"));
        }
        _ => {}
    }
    vec![node]
}

/// Sort les images d'un paragraphe : `[paragraphe sans images?, image, image…]`.
fn hoist_images(paragraph: PmNode) -> Vec<PmNode> {
    let has_image = paragraph
        .children()
        .iter()
        .any(|child| child.kind == "image");
    if !has_image {
        return vec![paragraph];
    }
    // [rust] `partition` sépare une collection en deux selon un prédicat.
    let (images, rest): (Vec<PmNode>, Vec<PmNode>) = paragraph
        .content
        .unwrap_or_default()
        .into_iter()
        .partition(|child| child.kind == "image");
    let mut blocks = Vec::new();
    if rest
        .iter()
        .any(|child| child.plain_text().trim() != "" || child.kind == "hardBreak")
    {
        let mut text_paragraph = PmNode::block("paragraph");
        text_paragraph.content = Some(rest);
        blocks.push(text_paragraph);
    }
    blocks.extend(images);
    blocks
}

fn trim_final_newline(code_block: &mut PmNode) {
    let Some(content) = &mut code_block.content else {
        return;
    };
    if let Some(text) = content.first_mut().and_then(|node| node.text.as_mut())
        && text.ends_with('\n')
    {
        text.pop();
    }
    // Un nœud texte vide est interdit par ProseMirror.
    content.retain(|node| node.text.as_deref() != Some(""));
    if content.is_empty() {
        code_block.content = None;
    }
}
