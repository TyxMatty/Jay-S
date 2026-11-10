//! The OCR that comes with Windows (`Windows.Media.Ocr`), as in Jay-E (`jay_docs::pdf_ocr`): reads in
//! the languages of the person's Windows, so a Portuguese Windows reads Portuguese.

/// The longest side the Windows OCR takes (`OcrEngine.MaxImageDimension` is 10 000 px).
const MAX_SIDE: u32 = 9_600;

/// The picture scaled down to fit the OCR, when needed.
pub fn fit(picture: image::RgbaImage) -> image::RgbaImage {
    let (w, h) = picture.dimensions();
    if w.max(h) <= MAX_SIDE {
        return picture;
    }
    let k = MAX_SIDE as f32 / w.max(h) as f32;
    image::imageops::resize(&picture, ((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1), image::imageops::FilterType::Triangle)
}

/// A word as the OCR placed it: its box in pixels of the picture, from the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A line as the OCR read it, and its words.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub words: Vec<Word>,
}

/// The text Windows reads in a picture (RGBA, `width` × `height`), one line per line. `Err` when
/// Windows has no OCR language installed.
pub fn recognize(width: u32, height: u32, rgba: &[u8]) -> Result<String, String> {
    Ok(text_of(&recognize_lines(width, height, rgba)?))
}

pub fn text_of(lines: &[Line]) -> String {
    lines.iter().flat_map(|l| [l.text.as_str(), "\n"]).collect()
}

/// The same reading with where each word is, for the PDF reader to match against the page.
#[cfg(windows)]
pub fn recognize_lines(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<Line>, String> {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Security::Cryptography::CryptographicBuffer;
    let e = |e: windows::core::Error| e.message();
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|_| "no OCR language is installed in Windows".to_owned())?;
    let mut bgra = rgba.to_vec();
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let buffer = CryptographicBuffer::CreateFromByteArray(&bgra).map_err(e)?;
    let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(&buffer, BitmapPixelFormat::Bgra8, width as i32, height as i32, BitmapAlphaMode::Premultiplied).map_err(e)?;
    let result = engine.RecognizeAsync(&bitmap).map_err(e)?.join().map_err(e)?;
    let mut lines = vec![];
    for line in result.Lines().map_err(e)? {
        let mut words = vec![];
        for word in line.Words().map_err(e)? {
            let r = word.BoundingRect().map_err(e)?;
            words.push(Word { text: word.Text().map_err(e)?.to_string_lossy(), x: r.X, y: r.Y, w: r.Width, h: r.Height });
        }
        lines.push(Line { text: line.Text().map_err(e)?.to_string_lossy(), words });
    }
    Ok(lines)
}

#[cfg(not(windows))]
pub fn recognize_lines(_: u32, _: u32, _: &[u8]) -> Result<Vec<Line>, String> {
    Err("OCR needs Windows".into())
}
