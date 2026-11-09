//! DOCX text: the paragraphs and tables of `word/document.xml`, with Word's automatic numbering
//! written out. Exams typed in Word often number questions and choices automatically, and then
//! "1." and "a)" aren't in the text at all, only in `word/numbering.xml`; without them the file would
//! look like content, not an exam.

use super::ExtractError;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::HashMap;
use std::io::Read;

/// The most one part may decompress to (a ZIP bomb stops here).
const MAX_PART_BYTES: u64 = 64 << 20;
const MAX_ENTRIES: usize = 20_000;

pub fn extract(bytes: &[u8]) -> Result<String, ExtractError> {
    let invalid = |e: &dyn std::fmt::Display| ExtractError::Invalid(e.to_string());
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| invalid(&e))?;
    if zip.len() > MAX_ENTRIES {
        return Err(ExtractError::Invalid("too many parts".into()));
    }
    let document = read_part(&mut zip, "word/document.xml")?.ok_or(ExtractError::Unsupported)?;
    let numbering = read_part(&mut zip, "word/numbering.xml")?.map(|xml| Numbering::parse(&xml)).unwrap_or_default();
    document_text(&document, numbering)
}

fn read_part(zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> Result<Option<String>, ExtractError> {
    let mut entry = match zip.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(ExtractError::Invalid(e.to_string())),
    };
    let mut data = Vec::new();
    // The declared size can lie: read one byte past the limit to know.
    (&mut entry).take(MAX_PART_BYTES + 1).read_to_end(&mut data).map_err(|e| ExtractError::Invalid(e.to_string()))?;
    if data.len() as u64 > MAX_PART_BYTES {
        return Err(ExtractError::Invalid(format!("{name} is too big once decompressed")));
    }
    String::from_utf8(data).map(Some).map_err(|_| ExtractError::Invalid(format!("{name} isn't UTF-8")))
}

fn attr(e: &BytesStart, name: &str) -> Option<String> {
    e.attributes().flatten().find(|a| a.key.local_name().as_ref() == name).and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok().map(|v| v.into_owned()))
}

/// A list level: how its number is written.
#[derive(Debug, Clone, Default)]
struct Level {
    format: String,
    text: String,
    start: u32,
}

#[derive(Debug, Default)]
struct Numbering {
    /// abstractNumId → its levels (0–8).
    abstracts: HashMap<String, Vec<Level>>,
    /// numId → abstractNumId.
    nums: HashMap<String, String>,
}

impl Numbering {
    fn parse(xml: &str) -> Self {
        let mut out = Self::default();
        let mut reader = Reader::from_str(xml);
        let (mut abstract_id, mut level, mut num_id): (Option<String>, Option<(usize, Level)>, Option<String>) = (None, None, None);
        loop {
            let (e, empty) = match reader.read_event() {
                Ok(Event::Start(e)) => (e, false),
                Ok(Event::Empty(e)) => (e, true),
                Ok(Event::End(e)) => {
                    match e.local_name().as_ref() {
                        "lvl" => {
                            if let (Some(id), Some((ilvl, lvl))) = (&abstract_id, level.take()) {
                                let levels = out.abstracts.entry(id.clone()).or_default();
                                if ilvl < 9 {
                                    if levels.len() <= ilvl {
                                        levels.resize(ilvl + 1, Level::default());
                                    }
                                    levels[ilvl] = lvl;
                                }
                            }
                        }
                        "abstractNum" => abstract_id = None,
                        "num" => num_id = None,
                        _ => {}
                    }
                    continue;
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => continue,
            };
            let _ = empty;
            match e.local_name().as_ref() {
                "abstractNum" => abstract_id = attr(&e, "abstractNumId"),
                "lvl" => level = attr(&e, "ilvl").and_then(|v| v.parse().ok()).map(|i| (i, Level { start: 1, ..Default::default() })),
                "start" => {
                    if let (Some((_, lvl)), Some(v)) = (&mut level, attr(&e, "val").and_then(|v| v.parse().ok())) {
                        lvl.start = v;
                    }
                }
                "numFmt" => {
                    if let (Some((_, lvl)), Some(v)) = (&mut level, attr(&e, "val")) {
                        lvl.format = v;
                    }
                }
                "lvlText" => {
                    if let (Some((_, lvl)), Some(v)) = (&mut level, attr(&e, "val")) {
                        lvl.text = v;
                    }
                }
                "num" => num_id = attr(&e, "numId"),
                "abstractNumId" => {
                    if let (Some(id), Some(v)) = (&num_id, attr(&e, "val")) {
                        out.nums.insert(id.clone(), v);
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn levels(&self, num_id: &str) -> Option<&Vec<Level>> {
        self.abstracts.get(self.nums.get(num_id)?)
    }
}

fn format_number(n: u32, format: &str) -> String {
    let letters = |upper: bool| {
        // a…z, then aa…zz, as Word does.
        let i = (n.max(1) - 1) as usize;
        let c = (b'a' + (i % 26) as u8) as char;
        let s: String = std::iter::repeat_n(c, i / 26 + 1).collect();
        if upper { s.to_uppercase() } else { s }
    };
    let roman = |upper: bool| {
        let mut n = n.min(3999);
        let mut s = String::new();
        for (v, r) in [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")] {
            while n >= v {
                s.push_str(r);
                n -= v;
            }
        }
        if upper { s.to_uppercase() } else { s }
    };
    match format {
        "lowerLetter" => letters(false),
        "upperLetter" => letters(true),
        "lowerRoman" => roman(false),
        "upperRoman" => roman(true),
        "decimalZero" => format!("{n:02}"),
        "bullet" => "•".into(),
        "none" => String::new(),
        _ => n.to_string(),
    }
}

fn document_text(xml: &str, numbering: Numbering) -> Result<String, ExtractError> {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut paragraph = String::new();
    let (mut in_text, mut num_id, mut ilvl): (bool, Option<String>, usize) = (false, None, 0);
    // (numId → counters per level).
    let mut counters: HashMap<String, [u32; 9]> = HashMap::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                "t" => in_text = true,
                "tab" => paragraph.push('\t'),
                "br" | "cr" => paragraph.push('\n'),
                "p" => (num_id, ilvl) = (None, 0),
                "numId" => num_id = attr(&e, "val").filter(|v| v != "0"),
                "ilvl" => ilvl = attr(&e, "val").and_then(|v| v.parse().ok()).unwrap_or(0).min(8),
                _ => {}
            },
            Ok(Event::Text(t)) if in_text => paragraph.push_str(&t.xml10_content()),
            Ok(Event::GeneralRef(r)) if in_text => match r.resolve_char_ref() {
                Ok(Some(c)) => paragraph.push(c),
                _ => paragraph.push_str(match r.xml10_content().as_ref() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    _ => "",
                }),
            },
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                "t" => in_text = false,
                "p" => {
                    let label = num_id.as_ref().and_then(|id| {
                        let levels = numbering.levels(id)?;
                        let lvl = levels.get(ilvl)?;
                        let counts = counters.entry(id.clone()).or_insert([0; 9]);
                        counts[ilvl] = if counts[ilvl] == 0 { lvl.start.max(1) } else { counts[ilvl] + 1 };
                        for deeper in counts.iter_mut().skip(ilvl + 1) {
                            *deeper = 0;
                        }
                        let mut text = lvl.text.clone();
                        for (i, l) in levels.iter().enumerate().take(ilvl + 1) {
                            text = text.replace(&format!("%{}", i + 1), &format_number(counts[i].max(l.start.max(1)), &l.format));
                        }
                        Some(text)
                    });
                    if let Some(label) = label.filter(|l| !l.is_empty()) {
                        out.push_str(&label);
                        out.push(' ');
                    }
                    out.push_str(&paragraph);
                    out.push('\n');
                    paragraph.clear();
                }
                "tc" => paragraph.push('\t'),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExtractError::Invalid(e.to_string())),
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    pub fn docx(document: &str, numbering: Option<&str>) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(document.as_bytes()).unwrap();
        if let Some(n) = numbering {
            zip.start_file("word/numbering.xml", opts).unwrap();
            zip.write_all(n.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        buf.into_inner()
    }

    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

    #[test]
    fn paragraphs_entities_and_automatic_numbering() {
        let p = |num: Option<(u32, u32)>, text: &str| {
            let pr = num.map(|(id, lvl)| format!("<w:pPr><w:numPr><w:ilvl w:val=\"{lvl}\"/><w:numId w:val=\"{id}\"/></w:numPr></w:pPr>")).unwrap_or_default();
            format!("<w:p>{pr}<w:r><w:t>{text}</w:t></w:r></w:p>")
        };
        let body = [p(Some((1, 0)), "Qual &amp; qual?"), p(Some((1, 1)), "Curitiba"), p(Some((1, 1)), "Londrina &#233;"), p(Some((1, 0)), "Outra"), p(Some((1, 1)), "Sim"), p(None, "Fim")].concat();
        let document = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
        let numbering = format!(
            r#"<w:numbering {NS}><w:abstractNum w:abstractNumId="7"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="%2)"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="7"/></w:num></w:numbering>"#
        );
        let text = extract(&docx(&document, Some(&numbering))).unwrap();
        assert_eq!(text, "1. Qual & qual?\na) Curitiba\nb) Londrina é\n2. Outra\na) Sim\nFim\n");
    }

    #[test]
    fn zip_without_a_document_is_unsupported() {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);
        zip.start_file("hello.txt", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"hi").unwrap();
        zip.finish().unwrap();
        assert_eq!(extract(&buf.into_inner()).unwrap_err(), ExtractError::Unsupported);
    }

    #[test]
    fn zip_bomb_is_refused() {
        // 65 MB of zeros compress to ~65 KB.
        let zeros = "0".repeat((MAX_PART_BYTES + 1024) as usize);
        let err = extract(&docx(&zeros, None)).unwrap_err();
        assert!(matches!(err, ExtractError::Invalid(ref m) if m.contains("too big")), "{err:?}");
    }
}
