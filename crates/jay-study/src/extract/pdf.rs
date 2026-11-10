//! PDF text with hayro. The glyph-collecting device and the line grouping are Jay-E's
//! (`jay_docs::pdf`), measured there on LaTeX and Word PDFs. A page with (almost) no text is a scan:
//! it's drawn and read by the Windows OCR.

use super::symbol::symbol;
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
/// Option letters one page can give back to its OCR text (a 90-question exam page has ~25).
const MAX_MARKERS: usize = 200;

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
        let PageText { text, unmapped, markers } = guard(|| page_text(page)).unwrap_or_else(|| {
            out.warnings.push(Warning::PageUnreadable { page: index as u32 + 1 });
            PageText::default()
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
                Some(pixmap) => match ocr::recognize_lines(pixmap.width() as u32, pixmap.height() as u32, pixmap.data_as_u8_slice()) {
                    Ok(mut lines) if ocr::text_of(&lines).chars().filter(|c| c.is_alphanumeric()).count() > letters => {
                        out.ocr_pages += 1;
                        restore_markers(&mut lines, &markers, scale);
                        ocr::text_of(&lines)
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

/// A lone option letter of the text layer ("A" to "E" alone on its line, or opening it: "A 12"):
/// its centre and size, in page points from the top-left. A sentence that opens with "A " or "E "
/// counts too; there the letter only replaces the same letter in the OCR text.
#[derive(Debug, Clone, PartialEq)]
struct Marker {
    letter: char,
    x: f32,
    y: f32,
    size: f32,
}

/// Gives the OCR text the option letters it can't read (white letters inside dark circles come out
/// as "O", "@" or nothing): each lone letter of the text layer takes the place of the short word
/// the OCR read over it, or goes before the nearest word to its right on the same row.
fn restore_markers(lines: &mut [ocr::Line], markers: &[Marker], scale: f32) {
    for m in markers.iter().take(MAX_MARKERS) {
        let (x, y, em) = (m.x * scale, m.y * scale, m.size * scale);
        let mut best: Option<(usize, usize, f32)> = None;
        for (li, line) in lines.iter().enumerate() {
            let top = line.words.iter().map(|w| w.y).fold(f32::MAX, f32::min);
            let bottom = line.words.iter().map(|w| w.y + w.h).fold(f32::MIN, f32::max);
            if !(top..=bottom).contains(&y) {
                continue;
            }
            for (wi, w) in line.words.iter().enumerate() {
                let gap = if (w.x..=w.x + w.w).contains(&x) { 0.0 } else { w.x - x };
                if (0.0..=4.0 * em).contains(&gap) && best.is_none_or(|b| gap < b.2) {
                    best = Some((li, wi, gap));
                }
            }
        }
        let Some((li, wi, gap)) = best else { continue };
        let line = &mut lines[li];
        if gap == 0.0 && line.words[wi].text.chars().count() <= 2 {
            line.words[wi].text = m.letter.to_string();
        } else {
            line.words.insert(wi, ocr::Word { text: m.letter.to_string(), x, y, w: 0.0, h: 0.0 });
        }
        line.text = line.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
    }
}

/// What the text layer of a page gives.
#[derive(Default)]
struct PageText {
    /// The text in drawing order, one line per line.
    text: String,
    /// Glyphs with no Unicode.
    unmapped: usize,
    markers: Vec<Marker>,
}

fn page_text(page: &hayro::hayro_syntax::page::Page<'_>) -> PageText {
    let cache = InterpreterCache::new();
    let (w, h) = page.render_dimensions();
    let mut ctx = Context::new(page.initial_transform(true).to_kurbo(), Rect::new(0.0, 0.0, w as f64, h as f64), &cache, page.xref(), InterpreterSettings::default());
    let mut device = TextDevice { glyphs: vec![], unmapped: 0 };
    interpret_page(page, &mut ctx, &mut device);
    let lines = lines_from_glyphs(&device.glyphs);
    let markers = lines
        .iter()
        .filter_map(|(text, g)| match text.trim().as_bytes() {
            [letter @ b'A'..=b'E'] | [letter @ b'A'..=b'E', b' ', ..] => {
                Some(Marker { letter: *letter as char, x: (g.x0 + g.x1) / 2.0, y: g.baseline - 0.35 * g.size, size: g.size })
            }
            _ => None,
        })
        .collect();
    PageText { text: lines.iter().map(|(text, _)| text.as_str()).collect::<Vec<_>>().join("\n"), unmapped: device.unmapped, markers }
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
            // The Symbol font says its glyphs are private-use characters: back to the letters.
            let text = if text.chars().any(|c| symbol(c).is_some()) { text.chars().map(|c| symbol(c).map(str::to_owned).unwrap_or_else(|| c.to_string())).collect() } else { text };
            if text.is_empty() {
                continue;
            }
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
/// Each line comes with its first glyph that isn't blank.
fn lines_from_glyphs(glyphs: &[PlacedGlyph]) -> Vec<(String, &PlacedGlyph)> {
    let same_line = |p: &PlacedGlyph, g: &PlacedGlyph| (g.baseline - p.baseline).abs() < 0.5 * p.size.min(g.size) && g.x0 > p.x0 - 0.5 * p.size;
    let mut order: Vec<&PlacedGlyph> = glyphs.iter().collect();
    reorder_equations(&mut order, same_line);
    let mut lines: Vec<(String, &PlacedGlyph)> = vec![];
    let mut last: Option<&PlacedGlyph> = None;
    for g in order {
        // Equation Editor draws each symbol twice in the same place (for weight): once is enough.
        if last.is_some_and(|p| p.text == g.text && (p.x0 - g.x0).abs() < 0.1 && (p.baseline - g.baseline).abs() < 0.1) {
            continue;
        }
        let same_line = last.is_some_and(|p| same_line(p, g));
        if !same_line {
            lines.push((String::new(), g));
        }
        let (line, first) = lines.last_mut().unwrap();
        if line.trim().is_empty() {
            *first = g;
        }
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
    lines.retain(|(l, _)| !l.trim().is_empty());
    lines
}

/// Glyphs a formula line can have and still be "a few symbols" (one line of "n =", "σ", "2 π").
const FORMULA_LINE: usize = 6;

/// An equation object (Word's Equation Editor) draws its symbols in no reading order: all the
/// digits, then the letters, then the operators, so "n = Z² p q / d²" comes out one symbol per
/// line. Where three or more lines of a few symbols follow each other, go back (up the page, or
/// to the left on the same row) and have a maths sign, the glyphs are put in reading order: rows
/// from the top, each from the left.
/// Fractions and exponents stay as rows; no structure is guessed.
fn reorder_equations<'a>(order: &mut [&'a PlacedGlyph], same_line: impl Fn(&PlacedGlyph, &PlacedGlyph) -> bool) {
    // Where each line starts, in drawing order.
    let mut starts: Vec<usize> = (0..order.len()).filter(|&i| i == 0 || !same_line(order[i - 1], order[i])).collect();
    starts.push(order.len());
    // A line of blanks only is not a line of the formula: it ends the run.
    let short = |l: usize| (1..=FORMULA_LINE).contains(&order[starts[l]..starts[l + 1]].iter().filter(|g| !g.text.trim().is_empty()).count());
    // An equation is a few rows tall: a line that starts far above or below the one before (the
    // next column, another part of the page) is not part of it.
    let near = |l: usize| l == 0 || (order[starts[l]].baseline - order[starts[l] - 1].baseline).abs() <= 4.0 * order[starts[l]].size;
    let mut blocks = vec![];
    let mut l = 0;
    while l + 1 < starts.len() {
        let mut end = l;
        while end + 1 < starts.len() && short(end) && (end == l || near(end)) {
            end += 1;
        }
        if end - l >= 3 {
            blocks.push(starts[l]..starts[end]);
        }
        l = end.max(l + 1);
    }
    for block in blocks {
        let glyphs = &mut order[block];
        let maths = glyphs.iter().any(|g| g.text.chars().any(|c| "=−±≤≥<>+×÷√∑∫".contains(c) || ('α'..='ω').contains(&c)));
        let goes_back = glyphs.windows(2).any(|w| {
            let half = 0.5 * w[0].size;
            w[1].baseline < w[0].baseline - half || ((w[1].baseline - w[0].baseline).abs() < half && w[1].x0 < w[0].x0 - half)
        });
        if !maths || !goes_back {
            continue;
        }
        glyphs.sort_by(|a, b| a.baseline.total_cmp(&b.baseline));
        // Rows: the next baseline down stays in the row when it is the same line (within 0.15 of
        // the size) or a smaller glyph up to 0.45 away, an exponent or an index. A numerator, a
        // denominator and the text line just below are full size: each gets its own row.
        let mut row_of = vec![0usize; glyphs.len()];
        for i in 1..glyphs.len() {
            let (p, g) = (glyphs[i - 1], glyphs[i]);
            let (gap, size) = (g.baseline - p.baseline, p.size.max(g.size));
            let script = p.size.min(g.size) < 0.85 * size;
            row_of[i] = if gap <= 0.15 * size || (script && gap <= 0.45 * size) { row_of[i - 1] } else { i };
        }
        let mut keyed: Vec<(usize, &PlacedGlyph)> = row_of.into_iter().zip(glyphs.iter().copied()).collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.x0.total_cmp(&b.1.x0)));
        for (slot, (_, g)) in glyphs.iter_mut().zip(keyed) {
            *slot = g;
        }
    }
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

    /// Glyphs of size 12, each 6 pt wide, at (x, baseline), in the order given.
    fn drawn(glyphs: &[(&str, f32, f32)]) -> Vec<String> {
        let glyphs: Vec<PlacedGlyph> = glyphs.iter().map(|&(text, x, baseline)| PlacedGlyph { text: text.into(), x0: x, x1: x + 6.0, baseline, size: 12.0 }).collect();
        lines_from_glyphs(&glyphs).into_iter().map(|(line, _)| line).collect()
    }

    #[test]
    fn an_equation_drawn_out_of_order_is_read_in_rows() {
        // "n = Z p q / d" as Equation Editor draws it: the denominator, the numerator from the
        // right, then the left side. One symbol per line before; rows from the top now.
        let fraction = [("d", 92.0, 112.0), ("q", 104.0, 90.0), ("p", 92.0, 90.0), ("Z", 80.0, 90.0), ("n", 50.0, 100.0), ("=", 62.0, 100.0)];
        assert_eq!(drawn(&fraction), ["Z p q", "n =", "d"]);
        // All on one row, drawn from the right: "x = 79 ± 3" in order.
        assert_eq!(drawn(&[("3", 110.0, 100.0), ("±", 98.0, 100.0), ("7", 80.0, 100.0), ("9", 86.0, 100.0), ("=", 68.0, 100.0), ("x", 56.0, 100.0)]), ["x = 79 ± 3"]);
        // Each symbol drawn twice in the same place counts once, even when the copies are apart.
        let twice = [("μ", 104.0, 90.0), ("=", 62.0, 100.0), ("μ", 104.0, 90.0), ("=", 62.0, 100.0), ("x", 50.0, 100.0), ("σ", 92.0, 112.0)];
        assert_eq!(drawn(&twice), ["μ", "x =", "σ"]);
    }

    #[test]
    fn short_lines_that_are_not_an_equation_keep_their_order() {
        // An answer key in two columns, drawn column by column: it goes back up, with no maths sign.
        let key: Vec<(&str, f32, f32)> = [(50.0, ["31", "32", "33"], ["C", "A", "D"]), (200.0, ["46", "47", "48"], ["B", "C", "A"])]
            .iter()
            .flat_map(|(x, numbers, letters)| (0..3).flat_map(move |i| [(numbers[i], *x, 100.0 + 20.0 * i as f32), (letters[i], x + 20.0, 100.0 + 20.0 * i as f32)]))
            .collect();
        assert_eq!(drawn(&key), ["31 C", "32 A", "33 D", "46 B", "47 C", "48 A"]);
        // Short lines with "=" that only go down the page: a list, left alone.
        let list = [("a", 50.0, 100.0), ("=", 62.0, 100.0), ("b", 50.0, 120.0), ("=", 62.0, 120.0), ("c", 50.0, 140.0), ("=", 62.0, 140.0), ("d", 50.0, 160.0)];
        assert_eq!(drawn(&list), ["a =", "b =", "c =", "d"]);
        // Two scrambled lines are not enough to call it an equation.
        assert_eq!(drawn(&[("=", 62.0, 100.0), ("x", 50.0, 100.0)]), ["=", "x"]);
        // A long line next to short ones is text: its words stay in drawing order.
        let text: Vec<(&str, f32, f32)> = "abcdefgh".split("").filter(|s| !s.is_empty()).enumerate().map(|(i, s)| (s, 50.0 + 6.0 * i as f32, 100.0)).collect();
        assert_eq!(drawn(&[text.as_slice(), &[("=", 62.0, 80.0), ("x", 50.0, 80.0)]].concat()), ["abcdefgh", "=", "x"]);
        // A UEPG proposition between two lines of blanks, the second one tiny and just below it:
        // blank lines are not formula lines, so "01) m < 0" is not taken apart (it was, once).
        let g = |text: &str, x: f32, baseline: f32, size: f32| PlacedGlyph { text: text.into(), x0: x, x1: x + 5.0, baseline, size };
        let label = [g(" ", 28.0, 269.3, 8.0), g("0", 51.0, 279.0, 8.0), g("1", 56.0, 279.0, 8.0), g(")", 61.0, 279.0, 8.0), g("m", 71.0, 279.0, 8.0), g("<", 82.0, 279.0, 8.0), g("0", 91.0, 279.0, 8.0), g(" ", 51.0, 282.6, 2.0)];
        assert_eq!(lines_from_glyphs(&label).into_iter().map(|(line, _)| line).collect::<Vec<_>>(), ["01) m < 0"]);
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
        let read = page_text(&pdf.pages()[0]);
        assert_eq!((read.text.as_str(), read.unmapped), ("", 15));
        // The same page in plain Helvetica loses nothing.
        let pdf = Pdf::new(sample(&[&["ABCABCABCABC", "CBA"]])).unwrap();
        let read = page_text(&pdf.pages()[0]);
        assert_eq!((read.text.as_str(), read.unmapped, read.markers.len()), ("ABCABCABCABC\nCBA", 0, 0));
    }

    /// An OCR line at row `y` (pixels): its words, each 40 px tall and as wide as given.
    fn ocr_line(y: f32, words: &[(&str, f32, f32)]) -> ocr::Line {
        let words: Vec<ocr::Word> = words.iter().map(|&(text, x, w)| ocr::Word { text: text.into(), x, y, w, h: 40.0 }).collect();
        ocr::Line { text: words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" "), words }
    }

    fn marker(letter: char, x: f32, y: f32) -> Marker {
        Marker { letter, x, y, size: 12.0 }
    }

    #[test]
    fn option_letters_go_back_to_their_own_row_or_nowhere() {
        // Markers are in page points; the page was drawn at 2 px per point.
        let page = || {
            vec![
                ocr_line(100.0, &[("combate", 100.0, 150.0), ("a", 260.0, 20.0), ("exclusão", 290.0, 160.0)]),
                ocr_line(160.0, &[("O", 100.0, 30.0), ("possibilita", 140.0, 200.0)]),
                ocr_line(220.0, &[("impede", 140.0, 120.0), ("que", 270.0, 60.0)]),
                ocr_line(280.0, &[("Oadmite", 100.0, 180.0), ("que", 290.0, 60.0)]),
                ocr_line(340.0, &[("obriga", 400.0, 120.0)]),
            ]
        };
        let mut lines = page();
        let markers = [
            marker('A', 57.0, 90.0),  // over the short word "O": takes its place
            marker('B', 57.0, 120.0), // nothing read over it: goes before "impede"
            marker('C', 57.0, 150.0), // over a long word: goes before it, the word stays
            marker('D', 57.0, 180.0), // the nearest word is more than 4 em away: nowhere
            marker('E', 57.0, 400.0), // no OCR line on that row: nowhere
            marker('A', f32::NAN, 90.0),
        ];
        restore_markers(&mut lines, &markers, 2.0);
        assert_eq!(ocr::text_of(&lines), "combate a exclusão\nA possibilita\nB impede que\nC Oadmite que\nobriga\n");
        // A row the OCR didn't read gets no letter from its neighbours.
        let mut lines = page();
        lines.remove(2);
        restore_markers(&mut lines, &markers, 2.0);
        assert_eq!(ocr::text_of(&lines), "combate a exclusão\nA possibilita\nC Oadmite que\nobriga\n");
        // No markers: the OCR text is untouched, byte for byte.
        let mut lines = vec![ocr::Line { text: "espaços   do  OCR".into(), words: vec![] }];
        restore_markers(&mut lines, &[], 2.0);
        assert_eq!(lines[0].text, "espaços   do  OCR");
        // A page can't ask for more letters than the cap.
        let mut lines: Vec<ocr::Line> = (0..300).map(|i| ocr_line(i as f32 * 60.0, &[("texto", 140.0, 100.0)])).collect();
        let many: Vec<Marker> = (0..300).map(|i| marker('A', 57.0, i as f32 * 30.0 + 10.0)).collect();
        restore_markers(&mut lines, &many, 2.0);
        assert_eq!(lines.iter().filter(|l| l.text == "A texto").count(), MAX_MARKERS);
    }

    #[test]
    fn markers_are_the_letters_alone_or_opening_a_line() {
        let pdf = Pdf::new(sample(&[&["Enunciado da questao", "A", "B 12", "Casa", "E agora", "F", "a", "AB"]])).unwrap();
        let read = page_text(&pdf.pages()[0]);
        assert_eq!(read.markers.iter().map(|m| m.letter).collect::<String>(), "ABE");
        // Each one sits on its own row, at the left margin of the sample page (x = 50 pt).
        assert!(read.markers.iter().all(|m| m.x > 50.0 && m.x < 60.0 && m.size == 12.0), "{:?}", read.markers);
        assert!(read.markers[0].y < read.markers[1].y && read.markers[1].y < read.markers[2].y);
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
