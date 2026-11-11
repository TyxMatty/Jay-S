//! Showing a material as it is (Materials › View), the way Jay-E does: PDF pages drawn by hayro
//! into PNGs one at a time, at the width the screen needs (only the pages near the screen are asked
//! for); DOCX through Jay-E's document model as a page of HTML (all text escaped); photos as they are.

use crate::extract::Format;
use crate::store::StoreError;
use crate::Store;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};
use serde::Serialize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

/// The widest page drawn (a 4K screen at 100%, or zoom on a smaller one).
pub const MAX_PAGE_WIDTH: u32 = 3000;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum View {
    /// Page sizes in points; pages come from the protocol one by one.
    Pdf { pages: Vec<[f32; 2]> },
    /// The document as HTML (text escaped by the writer); images through the protocol.
    Doc { html: String },
    Image,
    Text,
}

impl Store {
    /// The attached file as stored (`original.<ext>`).
    pub fn original(&self, material: &str) -> Result<(PathBuf, Format), StoreError> {
        let m = self.material(material)?;
        Ok((self.material_folder(material)?.join(format!("original.{}", m.format.extension())), m.format))
    }

    /// How the page should show `material`.
    pub fn view(&self, material: &str) -> Result<View, StoreError> {
        let (path, format) = self.original(material)?;
        Ok(match format {
            Format::Pdf => View::Pdf { pages: pdf_pages(std::fs::read(path)?) },
            Format::Docx => {
                let doc = crate::doc::docx::read(&std::fs::read(path)?).map_err(|e| StoreError::Extract(crate::ExtractError::Invalid(format!("{e:?}"))))?;
                let id = material.to_owned();
                View::Doc { html: crate::doc::html::to_html(&doc, &|i| format!("docimg/{id}/{i}")) }
            }
            Format::Image => View::Image,
            Format::Text => View::Text,
        })
    }

    /// Image `index` of a DOCX material, with its MIME type.
    pub fn doc_image(&self, material: &str, index: usize) -> Result<(Vec<u8>, &'static str), StoreError> {
        let (path, format) = self.original(material)?;
        if format != Format::Docx {
            return Err(StoreError::NotFound);
        }
        let doc = crate::doc::docx::read(&std::fs::read(path)?).map_err(|_| StoreError::NotFound)?;
        let image = doc.images.get(index).ok_or(StoreError::NotFound)?;
        Ok((image.bytes.clone(), crate::doc::image_mime(&image.bytes).ok_or(StoreError::NotFound)?))
    }
}

/// Page sizes of a PDF (empty when it can't be read).
pub fn pdf_pages(bytes: Vec<u8>) -> Vec<[f32; 2]> {
    catch_unwind(AssertUnwindSafe(|| Pdf::new(bytes).map(|pdf| pdf.pages().iter().map(|p| p.render_dimensions().into()).map(|(w, h): (f32, f32)| [w, h]).collect()).unwrap_or_default())).unwrap_or_default()
}

/// Page `index` drawn `width` pixels wide, as a PNG. `None` for a page that doesn't exist or can't
/// be drawn (a malformed page is caught, never a crash).
pub fn pdf_page_png(bytes: Vec<u8>, index: usize, width: u32) -> Option<Vec<u8>> {
    catch_unwind(AssertUnwindSafe(|| {
        let pdf = Pdf::new(bytes).ok()?;
        let page = pdf.pages().get(index)?;
        let (w, h) = page.render_dimensions();
        let scale = width.clamp(64, MAX_PAGE_WIDTH) as f32 / w.max(1.0);
        // At most ~25 Mpx, whatever the page's shape.
        let scale = scale.min((25_000_000.0 / (w * h).max(1.0)).sqrt());
        let settings = PixmapSettings { x_scale: scale, y_scale: scale, bg_color: WHITE };
        let pixmap = render(page, &RenderCache::new(), &InterpreterSettings::default(), &RenderSettings::default(), &settings);
        let (pw, ph) = (pixmap.width() as u32, pixmap.height() as u32);
        let rgba = image::RgbaImage::from_raw(pw, ph, pixmap.data_as_u8_slice().to_vec())?;
        let mut out = std::io::Cursor::new(Vec::new());
        // Fast compression: the protocol moves bytes, and a PNG is ~10x smaller than raw pixels (Jay-E).
        image::codecs::png::PngEncoder::new_with_quality(&mut out, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Up)
            .write_image(&rgba, pw, ph, image::ExtendedColorType::Rgba8)
            .ok()?;
        Some(out.into_inner())
    }))
    .ok()
    .flatten()
}

use image::ImageEncoder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_pages_draw_as_png_and_bad_pages_do_not() {
        let pdf = crate::extract::pdf_sample(&[&["Questao 1 Qual e a capital?"], &["Segunda pagina"]]);
        assert_eq!(pdf_pages(pdf.clone()), vec![[600.0, 800.0], [600.0, 800.0]]);
        let png = pdf_page_png(pdf.clone(), 1, 300).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        let decoded = image::load_from_memory(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (300, 400));
        assert!(pdf_page_png(pdf.clone(), 9, 300).is_none());
        assert!(pdf_page_png(b"%PDF-1.7 broken".to_vec(), 0, 300).is_none());
        // Width is capped.
        let big = image::load_from_memory(&pdf_page_png(pdf, 0, 999_999).unwrap()).unwrap();
        assert_eq!(big.width(), MAX_PAGE_WIDTH);
    }

    #[test]
    fn hostile_page_numbers_and_widths_stay_bounded() {
        let pdf = crate::extract::pdf_sample(&[&["Questao 1"]]);
        for index in [1usize, 1_000_000_000, u32::MAX as usize, usize::MAX] {
            assert!(pdf_page_png(pdf.clone(), index, 300).is_none(), "{index}");
        }
        // The widest and the narrowest asked for are clamped to [64, MAX_PAGE_WIDTH].
        let wide = image::load_from_memory(&pdf_page_png(pdf.clone(), 0, u32::MAX).unwrap()).unwrap();
        assert_eq!(wide.width(), MAX_PAGE_WIDTH);
        let thin = image::load_from_memory(&pdf_page_png(pdf, 0, 0).unwrap()).unwrap();
        assert_eq!(thin.width(), 64);
    }
}
