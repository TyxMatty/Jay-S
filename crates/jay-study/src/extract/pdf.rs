//! PDF text with hayro. The glyph-collecting device and the line grouping are Jay-E's
//! (`jay_docs::pdf`), measured there on LaTeX and Word PDFs. A page with (almost) no text is a scan:
//! it's drawn and read by the Windows OCR.

use super::{ExtractError, Extracted, Warning, ocr};
use hayro::hayro_interpret::font::{Glyph, GlyphRun};
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::kurbo::{Affine, BezPath, Point, Rect};
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageDrawProps, InterpreterCache, InterpreterSettings, SoftMask, TransformExt,
    interpret_page,
};
use hayro::hayro_syntax::{DecryptionError, LoadPdfError, Pdf};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Pages read from one PDF (a long textbook); the rest is skipped with a warning.
const MAX_PAGES: usize = 3_000;
/// Scanned pages read by OCR per PDF (each takes ~0.5–1 s).
const MAX_OCR_PAGES: u32 = 150;
/// A page with fewer letters than this is treated as a scan.
const SCAN_BELOW_CHARS: usize = 25;
/// Width a scanned page is drawn at for the OCR (~240 dpi on A4).
const OCR_WIDTH: f32 = 2_000.0;

pub fn extract(bytes: Vec<u8>) -> Result<Extracted, ExtractError> {
    let pdf = match catch_unwind(AssertUnwindSafe(|| Pdf::new(bytes))) {
        Ok(Ok(pdf)) => pdf,
        Ok(Err(LoadPdfError::Decryption(DecryptionError::PasswordProtected))) => return Err(ExtractError::Invalid("the PDF has a password".into())),
        _ => return Err(ExtractError::Invalid("not a valid PDF".into())),
    };
    let pages = pdf.pages();
    if pages.is_empty() {
        return Err(ExtractError::Invalid("the PDF has no pages".into()));
    }
    let mut out = Extracted::default();
    let mut ocr_error: Option<String> = None;
    let cache = RenderCache::new();
    for (index, page) in pages.iter().enumerate().take(MAX_PAGES) {
        let (text, unmapped) = guard(|| page_text(page)).unwrap_or_else(|| {
            out.warnings.push(Warning::PageUnreadable { page: index as u32 + 1 });
            (String::new(), 0)
        });
        let letters = text.chars().filter(|c| c.is_alphanumeric()).count();
        let text = if needs_ocr(letters, unmapped) && ocr_error.is_none() && out.ocr_pages < MAX_OCR_PAGES {
            let (w, h) = page.render_dimensions();
            let scale = OCR_WIDTH / w.max(1.0);
            let drawn = guard(|| {
                let settings = PixmapSettings { x_scale: scale, y_scale: scale, bg_color: WHITE };
                render(page, &cache, &InterpreterSettings::default(), &RenderSettings::default(), &settings)
            });
            match drawn.filter(|p| (p.width() as f32) < OCR_WIDTH * 1.1 && (p.height() as f32) < h * scale * 1.1 + 2.0) {
                Some(pixmap) => match ocr::recognize(pixmap.width() as u32, pixmap.height() as u32, pixmap.data_as_u8_slice()) {
                    Ok(read) if read.chars().filter(|c| c.is_alphanumeric()).count() > letters => {
                        out.ocr_pages += 1;
                        read
                    }
                    Ok(_) => text,
                    Err(e) => {
                        ocr_error = Some(e);
                        text
                    }
                },
                None => text,
            }
        } else {
            text
        };
        if index > 0 {
            out.text.push('\x0c');
        }
        out.text.push_str(&text);
        out.pages += 1;
    }
    if pages.len() > MAX_PAGES {
        out.warnings.push(Warning::PagesCut { kept: MAX_PAGES as u32 });
    }
    if let Some(e) = ocr_error {
        out.warnings.push(Warning::OcrUnavailable { detail: e });
    }
    Ok(out)
}

fn guard<T>(f: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(f)).ok()
}

/// Whether a page goes to the OCR: (almost) no text, or most of its glyphs in fonts that don't say
/// which letters they are (ENCCEJA 2017 and ENEM 2021 keep the headers and lose the questions).
fn needs_ocr(letters: usize, unmapped: usize) -> bool {
    letters < SCAN_BELOW_CHARS || (unmapped >= SCAN_BELOW_CHARS && unmapped > letters)
}

/// The page's text in drawing order, one line per line, and how many glyphs had no Unicode.
fn page_text(page: &hayro::hayro_syntax::page::Page<'_>) -> (String, usize) {
    let cache = InterpreterCache::new();
    let (w, h) = page.render_dimensions();
    let mut ctx = Context::new(page.initial_transform(true).to_kurbo(), Rect::new(0.0, 0.0, w as f64, h as f64), &cache, page.xref(), InterpreterSettings::default());
    let mut device = TextDevice { glyphs: vec![], unmapped: 0 };
    interpret_page(page, &mut ctx, &mut device);
    (lines_from_glyphs(&device.glyphs).join("\n"), device.unmapped)
}

/// A glyph as drawn: its text, box and baseline, in page points from the top-left.
struct PlacedGlyph {
    text: String,
    x0: f32,
    x1: f32,
    baseline: f32,
    size: f32,
}

/// Collects the glyphs of a page instead of drawing them.
struct TextDevice {
    glyphs: Vec<PlacedGlyph>,
    /// Glyphs dropped because the font gives no Unicode for them.
    unmapped: usize,
}

impl<'a> Device<'a> for TextDevice {
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'a>, _: &DrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_image(&mut self, _: Image<'a, '_>, _: ImageDrawProps<'a>) {}
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, 'a>, props: DrawProps<'a>, _: &DrawMode) {
        for glyph in run.glyphs() {
            let text = match glyph.as_unicode() {
                Some(BfString::Char(c)) => c.to_string(),
                Some(BfString::String(s)) => s,
                None => {
                    self.unmapped += 1;
                    continue;
                }
            };
            // Glyph space is 1000 units per em; the transform takes it to page points.
            let t: Affine = props.transform * glyph.transform();
            let advance = match &**glyph {
                Glyph::Outline(o) => o.advance_width().unwrap_or(500.0),
                Glyph::Type3(g) => g.advance_width().unwrap_or(500.0),
            } as f64;
            let origin = t * Point::ZERO;
            let end = t * Point::new(advance, 0.0);
            let up = t * Point::new(0.0, 1000.0);
            let size = ((up.x - origin.x).hypot(up.y - origin.y)) as f32;
            if !size.is_finite() || size <= 0.0 {
                continue;
            }
            let (x0, x1) = (origin.x.min(end.x) as f32, origin.x.max(end.x) as f32);
            self.glyphs.push(PlacedGlyph { text, x0, x1, baseline: origin.y as f32, size });
        }
    }
}

/// Groups glyphs in drawing order into lines: a new line when the baseline moves or the text goes
/// back to the left; a space where the gap between glyphs is wider than 0.12 em (Jay-E, measured).
fn lines_from_glyphs(glyphs: &[PlacedGlyph]) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let mut last: Option<&PlacedGlyph> = None;
    for g in glyphs {
        let same_line = last.is_some_and(|p| (g.baseline - p.baseline).abs() < 0.5 * p.size.min(g.size) && g.x0 > p.x0 - 0.5 * p.size);
        if !same_line {
            lines.push(String::new());
        }
        let line = lines.last_mut().unwrap();
        if let Some(p) = last.filter(|_| same_line)
            && g.x0 - p.x1 > 0.12 * g.size
            && !line.ends_with(' ')
            && !g.text.starts_with(' ')
        {
            line.push(' ');
        }
        line.push_str(&g.text);
        last = Some(g);
    }
    lines.retain(|l| !l.trim().is_empty());
    lines
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A PDF whose pages each draw the given lines in Helvetica, top to bottom.
    pub fn sample(pages: &[&[&str]]) -> Vec<u8> {
        sample_with_font("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>", pages)
    }

    fn sample_with_font(font: &str, pages: &[&[&str]]) -> Vec<u8> {
        let mut objs: Vec<String> = vec!["<< /Type /Catalog /Pages 2 0 R >>".into(), String::new(), font.into()];
        let mut kids = vec![];
        for lines in pages {
            let id = objs.len() + 1;
            kids.push(format!("{id} 0 R"));
            objs.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>", id + 1));
            let content: String = lines.iter().enumerate().map(|(i, l)| format!("BT /F1 12 Tf 50 {} Td ({}) Tj ET\n", 750 - 20 * i, l.replace('(', "\\(").replace(')', "\\)"))).collect();
            objs.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
        }
        objs[1] = format!("<< /Type /Pages /Count {} /Kids [{}] >>", pages.len(), kids.join(" "));
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offsets = vec![];
        for (i, o) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
        }
        let xref = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
        for o in offsets {
            out.extend(format!("{o:010} 00000 n \n").bytes());
        }
        out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).bytes());
        out
    }

    #[test]
    fn text_by_page_and_line() {
        let pdf = sample(&[&["Questao 1 Qual e a capital do Parana?", "a) Curitiba", "b) Ponta Grossa"], &["Segunda pagina com bastante texto aqui"]]);
        let out = extract(pdf).unwrap();
        assert_eq!(out.pages, 2);
        assert_eq!(out.text, "Questao 1 Qual e a capital do Parana?\na) Curitiba\nb) Ponta Grossa\x0cSegunda pagina com bastante texto aqui");
        assert_eq!(out.ocr_pages, 0);
    }

    #[test]
    fn a_page_of_glyphs_with_no_unicode_goes_to_the_ocr() {
        // No text, or more glyphs lost than letters read.
        for (letters, unmapped, ocr) in [(0, 0, true), (24, 0, true), (25, 0, false), (25, 24, false), (3000, 0, false), (3000, 2999, false), (3000, 3000, false), (100, 101, true), (120, 1500, true), (30, 26, false), (26, 30, true)] {
            assert_eq!(needs_ocr(letters, unmapped), ocr, "{letters} letters, {unmapped} unmapped");
        }
        // A font whose codes name glyphs nobody knows: the reader gets no letter and counts them.
        let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Type /Encoding /Differences [65 /g001 /g002 /g003] >> >>";
        let pdf = Pdf::new(sample_with_font(font, &[&["ABCABCABCABC", "CBA"]])).unwrap();
        assert_eq!(page_text(&pdf.pages()[0]), (String::new(), 15));
        // The same page in plain Helvetica loses nothing.
        let pdf = Pdf::new(sample(&[&["ABCABCABCABC", "CBA"]])).unwrap();
        assert_eq!(page_text(&pdf.pages()[0]), ("ABCABCABCABC\nCBA".to_owned(), 0));
    }

    #[test]
    fn truncated_pdf_reads_what_it_can_or_fails_cleanly() {
        let pdf = sample(&[&["Primeira pagina com texto suficiente para ler"], &["Segunda pagina com texto suficiente para ler"]]);
        let cut = pdf[..pdf.len() * 2 / 3].to_vec();
        // Either the first page comes out, or a clean error: never a crash.
        match extract(cut) {
            Ok(out) => assert!(out.pages >= 1),
            Err(ExtractError::Invalid(_)) => {}
            Err(e) => panic!("{e:?}"),
        }
    }
}
