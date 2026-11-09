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

/// The text Windows reads in a picture (RGBA, `width` × `height`), one line per line. `Err` when
/// Windows has no OCR language installed.
#[cfg(windows)]
pub fn recognize(width: u32, height: u32, rgba: &[u8]) -> Result<String, String> {
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
    let mut text = String::new();
    for line in result.Lines().map_err(e)? {
        text.push_str(&line.Text().map_err(e)?.to_string_lossy());
        text.push('\n');
    }
    Ok(text)
}

#[cfg(not(windows))]
pub fn recognize(_: u32, _: u32, _: &[u8]) -> Result<String, String> {
    Err("OCR needs Windows".into())
}
