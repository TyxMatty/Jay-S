//! DOCX shown as a page (Materials › View): Jay-E's document model, its DOCX reader and its HTML
//! writer (jay-docs/src/doc, copied without the ODT, RTF, Markdown and editing parts). The HTML
//! escapes all text and keeps only http(s)/mailto links; images go through the app protocol.

pub mod docx;
pub mod html;
mod xml;

use serde::Serialize;
use std::io::Read;
use std::path::Path;

/// Bounds for untrusted files (docs/ARQUITETURA.md, RNF-6).
pub const MAX_FILE_BYTES: u64 = 512 << 20;
/// One ZIP entry, uncompressed: more is a zip bomb.
pub const MAX_ENTRY_BYTES: u64 = 256 << 20;
/// Lists in lists, tables in tables, groups in RTF.
pub const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub images: Vec<Image>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Image {
    pub mime: String,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Justify,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    /// Level 1..=6.
    Heading { level: u8, inlines: Vec<Inline> },
    Paragraph { inlines: Vec<Inline>, align: Align },
    /// Each item is a list of blocks (a paragraph, maybe a nested list).
    List { ordered: bool, items: Vec<Vec<Block>> },
    /// Rows of cells; a cell is a list of blocks.
    Table { rows: Vec<Vec<Vec<Block>>> },
    Code { text: String },
    Quote { blocks: Vec<Block> },
    PageBreak,
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub code: bool,
    pub sup: bool,
    pub sub: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Inline {
    Text { text: String, style: Style },
    Link { href: String, inlines: Vec<Inline> },
    /// `index` into `Document::images`; the display size in points when the file says.
    Image { index: usize, width: Option<f32>, height: Option<f32> },
    /// A line break inside a paragraph.
    Break,
}

impl Inline {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into(), style: Style::default() }
    }
}

/// The plain text of some inlines (for headings in tables of contents, cells in Markdown…).
pub fn plain(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for i in inlines {
        match i {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::Link { inlines, .. } => out.push_str(&plain(inlines)),
            Inline::Break => out.push('\n'),
            Inline::Image { .. } => {}
        }
    }
    out
}

/// Joins neighbouring text runs with the same style ("Hel" + "lo" → "Hello"): readers split runs
/// wherever the source did (spell checks, revisions), which writers don't need.
pub fn merge_runs(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match (out.last_mut(), inline) {
            (Some(Inline::Text { text, style }), Inline::Text { text: more, style: s }) if *style == s => text.push_str(&more),
            (_, Inline::Text { text, .. }) if text.is_empty() => {}
            (_, other) => out.push(other),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DocError {
    TooBig,
    Unsupported,
    /// Changed on disk since it was opened: saving would lose those changes.
    Changed,
    Invalid { message: String },
    Io { message: String },
}

impl From<std::io::Error> for DocError {
    fn from(e: std::io::Error) -> Self {
        Self::Io { message: e.to_string() }
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> DocError {
    DocError::Invalid { message: message.into() }
}

/// Reads a document by its detected format ("docx", "odt", "rtf", "markdown", "html").
pub fn read(path: &Path, format: &str) -> Result<Document, DocError> {
    let len = std::fs::metadata(path)?.len();
    if len > MAX_FILE_BYTES {
        return Err(DocError::TooBig);
    }
    let bytes = std::fs::read(path)?;
    match format {
        "docx" => docx::read(&bytes),
        _ => Err(DocError::Unsupported),
    }
}

/// One entry of a ZIP (DOCX, ODT), refusing more than `MAX_ENTRY_BYTES` whatever the header says.
pub(crate) fn zip_entry<R: std::io::Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>, name: &str) -> Result<Option<Vec<u8>>, DocError> {
    let file = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(invalid(e.to_string())),
    };
    let mut out = Vec::new();
    file.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut out)?;
    if out.len() as u64 > MAX_ENTRY_BYTES {
        return Err(DocError::TooBig);
    }
    Ok(Some(out))
}

/// The MIME type of image bytes, from their signature (DOCX/ODT name them, but names lie).
pub(crate) fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    let b = bytes;
    Some(if b.starts_with(b"\x89PNG") {
        "image/png"
    } else if b.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if b.starts_with(b"GIF8") {
        "image/gif"
    } else if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        "image/webp"
    } else if b.starts_with(b"BM") {
        "image/bmp"
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_merge_when_styles_match() {
        let bold = Style { bold: true, ..Default::default() };
        let runs = vec![
            Inline::text("Hel"),
            Inline::text("lo "),
            Inline::Text { text: "world".into(), style: bold },
            Inline::Text { text: "".into(), style: bold },
            Inline::Text { text: "!".into(), style: bold },
            Inline::Break,
        ];
        assert_eq!(merge_runs(runs), vec![Inline::text("Hello "), Inline::Text { text: "world!".into(), style: bold }, Inline::Break]);
    }
}
