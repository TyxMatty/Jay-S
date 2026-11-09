//! The text of an attached file: PDF (hayro, with the Windows OCR for scanned pages), DOCX, plain
//! text/Markdown, and photos (Windows OCR). Every file is hostile (docs/ARQUITETURA.md, RNF-5): the
//! format comes from the bytes, sizes are capped, and a reader that panics is caught and becomes an
//! error for that file.

mod docx;
mod ocr;
mod pdf;

#[cfg(test)]
pub(crate) use pdf::tests::sample as pdf_sample;

use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The biggest file Jay-S takes.
pub const MAX_FILE_BYTES: u64 = 200 << 20;
/// The most text kept from one file (a few textbooks); the rest is dropped with a warning.
pub const MAX_TEXT_CHARS: usize = 4_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Docx,
    Text,
    Image,
}

impl Format {
    /// The extension the stored copy gets.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Text => "txt",
            Self::Image => "img",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum ExtractError {
    TooBig,
    /// Not a format Jay-S reads (the bytes say so, whatever the extension).
    Unsupported,
    /// The file is broken or hostile.
    Invalid(String),
    Io(String),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooBig => write!(f, "file too big (max {} MB)", MAX_FILE_BYTES >> 20),
            Self::Unsupported => write!(f, "not a PDF, DOCX, text file or photo"),
            Self::Invalid(why) => write!(f, "unreadable file: {why}"),
            Self::Io(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for ExtractError {}

/// Something the student should know about a file that was read anyway. The page translates each.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Warning {
    /// A page that couldn't be read (1-based).
    PageUnreadable { page: u32 },
    /// Only the first `kept` pages were read.
    PagesCut { kept: u32 },
    /// Scanned pages couldn't be read by the OCR (no OCR language in Windows, usually).
    OcrUnavailable { detail: String },
    /// Only the beginning of the text was kept.
    TextCut,
    NoTextInPhoto,
}

/// What was read from a file. `text` keeps page breaks as form feeds (`\f`).
#[derive(Debug, Clone, Default)]
pub struct Extracted {
    pub text: String,
    pub pages: u32,
    /// Pages (or photos) read by OCR.
    pub ocr_pages: u32,
    pub warnings: Vec<Warning>,
}

/// The format, from the first bytes. The extension only decides between text and nothing.
pub fn detect(head: &[u8]) -> Option<Format> {
    let start = &head[..head.len().min(1024)];
    if start.windows(5).any(|w| w == b"%PDF-") {
        return Some(Format::Pdf);
    }
    if head.starts_with(b"PK\x03\x04") {
        return Some(Format::Docx);
    }
    if head.starts_with(b"\x89PNG\r\n\x1a\n") || head.starts_with(b"\xFF\xD8\xFF") {
        return Some(Format::Image);
    }
    let utf16 = head.starts_with(b"\xFF\xFE") || head.starts_with(b"\xFE\xFF");
    // Text: no NUL bytes (binary files nearly always have them), unless it's UTF-16.
    (utf16 || !head.iter().take(4096).any(|&b| b == 0)).then_some(Format::Text)
}

/// Reads `bytes` as `format`. Panics in a reader are caught.
pub fn extract(format: Format, bytes: Vec<u8>) -> Result<Extracted, ExtractError> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ExtractError::TooBig);
    }
    let result = catch_unwind(AssertUnwindSafe(|| match format {
        Format::Pdf => pdf::extract(bytes),
        Format::Docx => docx::extract(&bytes).map(|text| Extracted { text, pages: 1, ..Default::default() }),
        Format::Text => Ok(Extracted { text: decode_text(&bytes), pages: 1, ..Default::default() }),
        Format::Image => image_text(&bytes),
    }));
    let mut out = match result {
        Ok(r) => r?,
        Err(_) => return Err(ExtractError::Invalid("the reader stopped on a malformed part".into())),
    };
    out.text = out.text.replace("\r\n", "\n").replace('\r', "\n");
    if out.text.chars().count() > MAX_TEXT_CHARS {
        out.text = out.text.chars().take(MAX_TEXT_CHARS).collect();
        out.warnings.push(Warning::TextCut);
    }
    Ok(out)
}

/// UTF-8 (with or without BOM), UTF-16 with BOM, and otherwise Windows-1252 (old Windows text files
/// in Portuguese).
pub fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let utf16 = |rest: &[u8], le: bool| {
        let units: Vec<u16> = rest.chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
        String::from_utf16_lossy(&units)
    };
    if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        return utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        return utf16(rest, false);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| windows_1252(b)).collect(),
    }
}

fn windows_1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›',
        'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9f => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// The biggest photo decoded (pixels and decoder memory).
const MAX_IMAGE_PIXELS: u64 = 80_000_000;

fn image_text(bytes: &[u8]) -> Result<Extracted, ExtractError> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e| ExtractError::Invalid(e.to_string()))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_IMAGE_PIXELS * 4 + (64 << 20));
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    reader.limits(limits);
    let picture = reader.decode().map_err(|e| ExtractError::Invalid(e.to_string()))?.into_rgba8();
    if picture.width() as u64 * picture.height() as u64 > MAX_IMAGE_PIXELS {
        return Err(ExtractError::TooBig);
    }
    // The Windows OCR reads at most ~10 000 px per side; bigger photos are scaled down first.
    let picture = ocr::fit(picture);
    let text = ocr::recognize(picture.width(), picture.height(), picture.as_raw()).map_err(ExtractError::Invalid)?;
    let mut out = Extracted { text, pages: 1, ocr_pages: 1, ..Default::default() };
    if out.text.trim().is_empty() {
        out.warnings.push(Warning::NoTextInPhoto);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_comes_from_the_bytes() {
        assert_eq!(detect(b"%PDF-1.7\n..."), Some(Format::Pdf));
        assert_eq!(detect(b"\xEF\xBB\xBF%PDF-1.4"), Some(Format::Pdf));
        assert_eq!(detect(b"PK\x03\x04rest"), Some(Format::Docx));
        assert_eq!(detect(b"\x89PNG\r\n\x1a\nIHDR"), Some(Format::Image));
        assert_eq!(detect(b"\xFF\xD8\xFF\xE0JFIF"), Some(Format::Image));
        assert_eq!(detect("Questão 1\na) sim".as_bytes()), Some(Format::Text));
        assert_eq!(detect(b"\xFF\xFEQ\0u\0"), Some(Format::Text));
        // A program or any other binary file.
        assert_eq!(detect(b"MZ\x90\0\x03\0\0\0"), None);
    }

    #[test]
    fn text_encodings() {
        assert_eq!(decode_text("Função".as_bytes()), "Função");
        assert_eq!(decode_text(b"\xEF\xBB\xBFsim"), "sim");
        assert_eq!(decode_text(b"Fun\xe7\xe3o \x93x\x94"), "Função “x”");
        assert_eq!(decode_text(b"\xFF\xFEo\0i\0"), "oi");
        assert_eq!(decode_text(b"\xFE\xFF\0o\0i"), "oi");
    }

    #[test]
    fn text_files_normalize_line_ends() {
        let out = extract(Format::Text, b"a\r\nb\rc".to_vec()).unwrap();
        assert_eq!(out.text, "a\nb\nc");
    }

    #[test]
    fn broken_files_are_errors_not_crashes() {
        assert!(matches!(extract(Format::Pdf, b"%PDF-1.7\n1 0 obj << /Type /Catalog".to_vec()), Err(ExtractError::Invalid(_))));
        assert!(matches!(extract(Format::Docx, b"PK\x03\x04garbage".to_vec()), Err(ExtractError::Invalid(_))));
        assert!(matches!(extract(Format::Image, b"\x89PNG\r\n\x1a\nbroken".to_vec()), Err(ExtractError::Invalid(_))));
    }

    #[test]
    fn huge_files_are_refused() {
        assert_eq!(extract(Format::Text, vec![b'a'; MAX_FILE_BYTES as usize + 1]).unwrap_err(), ExtractError::TooBig);
    }
}
