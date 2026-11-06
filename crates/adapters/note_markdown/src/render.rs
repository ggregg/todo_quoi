//! Document TipTap → Markdown étendu, sous une **forme canonique** : relire puis réécrire
//! un fichier déjà canonique doit redonner exactement le même texte.
//!
//! Choix d'écriture : `**gras**`, `*italique*`, `~~barré~~`, `` `code` ``, listes avec `-`,
//! blocs séparés par une ligne vide, caractères spéciaux échappés par `\`.

use crate::directives::render_attrs;
use crate::pm::{PmMark, PmNode};

/// Convertit un document TipTap en Markdown (terminé par un saut de ligne, ou vide).
///
/// Les paragraphes vides de fin sont ignorés : l'éditeur en garde toujours un après un bloc de
/// code ou un tableau (pour pouvoir y placer le curseur), mais ils n'ont pas de sens dans le
/// fichier (constat de l'I2 : le fichier se terminait par des lignes vides).
pub fn doc_to_markdown(doc: &PmNode) -> String {
    let blocks = doc.children();
    let meaningful = blocks
        .iter()
        .rposition(|block| !is_empty_paragraph(block))
        .map_or(0, |last| last + 1);
    let body = render_blocks(&blocks[..meaningful]);
    if body.trim().is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

fn is_empty_paragraph(block: &PmNode) -> bool {
    block.kind == "paragraph"
        && block.plain_text().trim().is_empty()
        && !block.children().iter().any(|c| c.kind != "text")
}

/// Rend une suite de blocs, séparés par une ligne vide.
fn render_blocks(blocks: &[PmNode]) -> String {
    blocks
        .iter()
        .map(render_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_block(node: &PmNode) -> String {
    match node.kind.as_str() {
        "paragraph" => escape_block_start(&render_inline(node.children())),
        "heading" => {
            let level = node.attr_u64("level").unwrap_or(1).clamp(1, 6) as usize;
            format!("{} {}", "#".repeat(level), render_inline(node.children()))
        }
        "blockquote" => prefix_lines(&render_blocks(node.children()), "> ", ">"),
        "codeBlock" => {
            let language = node.attr_str("language").unwrap_or("");
            format!("```{language}\n{}\n```", node.plain_text())
        }
        "horizontalRule" => "---".to_string(),
        "bulletList" => render_list(node, |_, _| "- ".to_string()),
        "taskList" => render_list(node, |_, item| {
            if item.attr_bool("checked") {
                "- [x] "
            } else {
                "- [ ] "
            }
            .to_string()
        }),
        "orderedList" => {
            let start = node.attr_u64("start").unwrap_or(1);
            render_list(node, |index, _| format!("{}. ", start + index))
        }
        "table" => render_table(node),
        "image" => render_image(node),
        "radioGroup" => render_radio_group(node),
        "audio" => format!(
            "::audio{}",
            render_attrs(node.attrs.as_ref(), &["src", "duration", "mime"])
        ),
        "attachment" => format!(
            "::file{}",
            render_attrs(node.attrs.as_ref(), &["src", "name", "size"])
        ),
        other => format!("<!-- bloc non pris en charge : {other} -->"),
    }
}

// ----- Listes -------------------------------------------------------------------------

/// Rend une liste ; `marker(index, élément)` donne le préfixe de chaque élément.
// [rust] `impl Fn(u64, &PmNode) -> String` : « n'importe quelle fonction de cette forme »
//        (≈ un paramètre `(index: number, item: Node) => string` en TS).
fn render_list(list: &PmNode, marker: impl Fn(u64, &PmNode) -> String) -> String {
    list.children()
        .iter()
        .enumerate()
        .map(|(index, item)| render_list_item(item, &marker(index as u64, item)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_list_item(item: &PmNode, marker: &str) -> String {
    let children = item.children();
    let mut body = String::new();
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            // Une sous-liste se colle à la ligne précédente ; un autre bloc est séparé par une ligne vide.
            body.push_str(if is_list(child) { "\n" } else { "\n\n" });
        }
        body.push_str(&render_block(child));
    }
    // Les lignes suivantes sont indentées sous le texte de l'élément. Pour une case à
    // cocher, on indente sous le « - » (2 espaces), pas sous le « [ ] ».
    let indent = if marker.starts_with("- [") {
        2
    } else {
        marker.len()
    };
    let mut lines = body.lines();
    let first = lines.next().unwrap_or("");
    let mut out = format!("{marker}{first}").trim_end().to_string();
    for line in lines {
        out.push('\n');
        if !line.is_empty() {
            out.push_str(&" ".repeat(indent));
            out.push_str(line);
        }
    }
    out
}

fn is_list(node: &PmNode) -> bool {
    matches!(
        node.kind.as_str(),
        "bulletList" | "orderedList" | "taskList"
    )
}

// ----- Tableaux, images, radios -------------------------------------------------------

fn render_table(table: &PmNode) -> String {
    let rows: Vec<Vec<String>> = table
        .children()
        .iter()
        .map(|row| row.children().iter().map(render_cell).collect())
        .collect();
    let Some(first_row) = rows.first() else {
        return String::new();
    };
    let separator = vec!["---".to_string(); first_row.len()];
    let mut lines = vec![table_line(first_row), table_line(&separator)];
    lines.extend(rows.iter().skip(1).map(|row| table_line(row)));
    lines.join("\n")
}

fn render_cell(cell: &PmNode) -> String {
    let paragraphs: Vec<String> = cell
        .children()
        .iter()
        .map(|p| render_inline(p.children()))
        .collect();
    paragraphs.join(" ")
}

fn table_line(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

fn render_image(image: &PmNode) -> String {
    let alt = escape_inline(image.attr_str("alt").unwrap_or(""));
    let src = image.attr_str("src").unwrap_or("");
    let title = image
        .attr_str("title")
        .map(|t| format!(" \"{}\"", t.replace('"', "\\\"")))
        .unwrap_or_default();
    let has_size = image.attr("width").is_some() || image.attr("height").is_some();
    let size = if has_size {
        // Seules la largeur et la hauteur sont écrites, pas `src`/`alt`/`title` (déjà dans le lien).
        let mut size_only = image.clone();
        if let Some(attrs) = &mut size_only.attrs {
            attrs.retain(|key, _| key == "width" || key == "height");
        }
        render_attrs(size_only.attrs.as_ref(), &["width", "height"])
    } else {
        String::new()
    };
    format!("![{alt}]({src}{title}){size}")
}

fn render_radio_group(group: &PmNode) -> String {
    let mut lines = vec![format!(
        ":::radio{}",
        render_attrs(group.attrs.as_ref(), &["label"])
    )];
    for option in group.children() {
        let state = if option.attr_bool("checked") {
            "x"
        } else {
            " "
        };
        lines.push(format!("- ({state}) {}", option.plain_text()));
    }
    lines.push(":::".to_string());
    lines.join("\n")
}

// ----- Texte en ligne et marques ------------------------------------------------------

/// Ordre canonique d'ouverture des marques (de l'extérieur vers l'intérieur).
const MARK_ORDER: [&str; 5] = ["bold", "italic", "strike", "link", "code"];

/// Marques écrites avec des délimiteurs (`**`, `*`, `~~`) qui ne tolèrent pas d'espace
/// juste à l'intérieur : `**gras **` n'est PAS du gras en CommonMark.
const DELIMITED_MARKS: [&str; 3] = ["bold", "italic", "strike"];

/// Sort les espaces situés **là où une marque gras / italique / barré commence ou finit** :
/// `[« gras  » en gras][« suite »]` devient `[« gras » en gras][«   »][« suite »]`.
///
/// Constat du POC I0-1 : sans cela, une sélection TipTap qui inclut l'espace suivant
/// produit `**gras **suite`, et la mise en forme est perdue à la relecture.
/// Un espace à l'intérieur d'une marque qui continue sur le nœud voisin est conservé.
fn expel_edge_whitespace(nodes: &[PmNode]) -> Vec<PmNode> {
    let mut out = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        let Some(text) = node.text.as_deref() else {
            out.push(node.clone());
            continue;
        };
        // [rust] `index.checked_sub(1)` : `None` au lieu d'un dépassement si index == 0.
        let previous = index.checked_sub(1).and_then(|i| nodes.get(i));
        let next = nodes.get(index + 1);
        let ending = marks_ending_here(node, next);
        let starting = marks_ending_here(node, previous);

        let start = if starting.is_empty() {
            0
        } else {
            text.len() - text.trim_start().len()
        };
        let end = if ending.is_empty() {
            text.len()
        } else {
            text.trim_end().len().max(start)
        };
        if start > 0 {
            out.push(without_marks(node, &text[..start], &starting));
        }
        if end > start {
            out.push(PmNode::text(&text[start..end], node.marks()));
        }
        if end < text.len() {
            out.push(without_marks(node, &text[end..], &ending));
        }
    }
    out
}

/// Marques délimitées de `node` absentes du voisin : elles s'ouvrent ou se ferment ici.
fn marks_ending_here(node: &PmNode, neighbour: Option<&PmNode>) -> Vec<String> {
    let neighbour_marks = neighbour.map(PmNode::marks).unwrap_or(&[]);
    node.marks()
        .iter()
        .filter(|mark| DELIMITED_MARKS.contains(&mark.kind.as_str()))
        .filter(|mark| !neighbour_marks.contains(mark))
        .map(|mark| mark.kind.clone())
        .collect()
}

fn without_marks(node: &PmNode, text: &str, removed: &[String]) -> PmNode {
    let marks: Vec<PmMark> = node
        .marks()
        .iter()
        .filter(|mark| !removed.contains(&mark.kind))
        .cloned()
        .collect();
    PmNode::text(text, &marks)
}

/// Rend une suite de nœuds en ligne en ouvrant / fermant les marques au fil du texte.
///
/// Les marques actives sont gardées dans une pile : quand le nœud suivant n'a plus une
/// marque, on referme cette marque et toutes celles ouvertes après elle.
fn render_inline(nodes: &[PmNode]) -> String {
    let mut out = String::new();
    let mut open: Vec<PmMark> = Vec::new();
    for node in &expel_edge_whitespace(nodes) {
        if node.kind == "hardBreak" {
            out.push_str("\\\n");
            continue;
        }
        let wanted = sorted_marks(node.marks());
        let common = open.iter().zip(&wanted).take_while(|(a, b)| a == b).count();
        while open.len() > common {
            if let Some(mark) = open.pop() {
                out.push_str(&closing(&mark));
            }
        }
        for mark in &wanted[common..] {
            out.push_str(opening(mark));
            open.push(mark.clone());
        }
        let text = node.text.as_deref().unwrap_or("");
        let is_code = wanted.iter().any(|mark| mark.kind == "code");
        out.push_str(&if is_code {
            text.to_string()
        } else {
            escape_inline(text)
        });
    }
    while let Some(mark) = open.pop() {
        out.push_str(&closing(&mark));
    }
    out
}

fn sorted_marks(marks: &[PmMark]) -> Vec<PmMark> {
    let rank = |mark: &PmMark| {
        MARK_ORDER
            .iter()
            .position(|k| *k == mark.kind)
            .unwrap_or(MARK_ORDER.len())
    };
    let mut sorted: Vec<PmMark> = marks
        .iter()
        .filter(|mark| MARK_ORDER.contains(&mark.kind.as_str())) // marques sans équivalent Markdown ignorées
        .cloned()
        .collect();
    sorted.sort_by_key(rank);
    sorted
}

fn opening(mark: &PmMark) -> &'static str {
    match mark.kind.as_str() {
        "bold" => "**",
        "italic" => "*",
        "strike" => "~~",
        "link" => "[",
        "code" => "`",
        _ => "",
    }
}

fn closing(mark: &PmMark) -> String {
    match mark.kind.as_str() {
        "link" => {
            let href = mark.attr_str("href").unwrap_or("");
            match mark.attr_str("title") {
                Some(title) => format!("]({href} \"{}\")", title.replace('"', "\\\"")),
                None => format!("]({href})"),
            }
        }
        other => opening(&PmMark::new(other)).to_string(),
    }
}

/// Échappe les caractères qui auraient un sens en Markdown.
fn escape_inline(text: &str) -> String {
    const SPECIAL: &str = "\\`*_[]<>{}~|#&";
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if SPECIAL.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Un paragraphe qui commence par `- `, `+`, `=` ou `1.` serait lu comme autre chose.
fn escape_block_start(line: &str) -> String {
    if line.starts_with(['-', '+', '=']) {
        return format!("\\{line}");
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let after_digits = line[digits..].chars().next();
    if digits > 0 && matches!(after_digits, Some('.') | Some(')')) {
        return format!("{}\\{}", &line[..digits], &line[digits..]);
    }
    line.to_string()
}

fn prefix_lines(text: &str, prefix: &str, empty_prefix: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                empty_prefix.to_string()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::markdown_to_doc;
    use serde_json::json;

    fn doc_from_json(value: serde_json::Value) -> PmNode {
        serde_json::from_value(value).expect("JSON de test valide")
    }

    #[test]
    fn edge_spaces_are_moved_outside_bold_and_italic() {
        let doc = doc_from_json(
            json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [
                { "type": "text", "text": "Du " },
                { "type": "text", "text": "gras ", "marks": [{ "type": "bold" }] },
                { "type": "text", "text": "puis ", "marks": [{ "type": "italic" }] },
                { "type": "text", "text": "fin" }
            ]}]}),
        );
        let markdown = doc_to_markdown(&doc);
        assert_eq!(markdown, "Du **gras** *puis* fin\n");

        // Relu, le texte en gras est bien « gras » (sans l'espace) : la mise en forme a survécu.
        let reread = markdown_to_doc(&markdown);
        let bold = &reread.children()[0].children()[1];
        assert_eq!(bold.text.as_deref(), Some("gras"));
        assert_eq!(bold.marks()[0].kind, "bold");
    }
}
