//! Materials › View: the `jays://` protocol that serves what the page shows, like Jay-E's `jaye://`.
//! Only ids go in the address, checked by the store; nothing outside the data folder is ever read.
//!
//! - `/pdf/<material>/<page>/<width>`: a PDF page as PNG, drawn when asked (only pages near the screen)
//! - `/docimg/<material>/<index>`: an image inside a DOCX
//! - `/file/<material>`: an attached photo

use crate::library::{Failure, Library};
use jay_study::Format;
use jay_study::view::View;
use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, Runtime, UriSchemeContext, UriSchemeResponder};

fn reply(status: StatusCode, mime: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        // Dev pages come from http://localhost:5178, a different origin.
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap()
}

fn not_found() -> Response<Vec<u8>> {
    reply(StatusCode::NOT_FOUND, "text/plain", vec![])
}

pub fn protocol<R: Runtime>(ctx: UriSchemeContext<'_, R>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().to_owned();
    // Drawing a page takes tens of milliseconds: off the protocol's thread.
    std::thread::spawn(move || {
        let library = app.state::<Library>();
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let response = match parts.as_slice() {
            ["pdf", id, page, width] => match (library.store.original(id), page.parse::<usize>(), width.parse::<u32>()) {
                (Ok((file, Format::Pdf)), Ok(page), Ok(width)) => std::fs::read(file)
                    .ok()
                    .and_then(|bytes| jay_study::view::pdf_page_png(bytes, page, width))
                    .map(|png| reply(StatusCode::OK, "image/png", png))
                    .unwrap_or_else(|| reply(StatusCode::NO_CONTENT, "image/png", vec![])),
                _ => not_found(),
            },
            ["docimg", id, index] => match index.parse::<usize>().ok().and_then(|i| library.store.doc_image(id, i).ok()) {
                Some((bytes, mime)) => reply(StatusCode::OK, mime, bytes),
                None => not_found(),
            },
            ["file", id] => match library.store.original(id) {
                Ok((file, Format::Image)) => match std::fs::read(file) {
                    Ok(bytes) => {
                        let mime = if bytes.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
                        reply(StatusCode::OK, mime, bytes)
                    }
                    Err(_) => not_found(),
                },
                _ => not_found(),
            },
            _ => not_found(),
        };
        responder.respond(response);
    });
}

/// How the page should show a material (PDF pages, DOCX as HTML, a photo, text).
#[tauri::command]
pub async fn materials_view(app: tauri::AppHandle, id: String) -> Result<View, Failure> {
    // async: reading a DOCX or counting a PDF's pages shouldn't hold the window.
    tauri::async_runtime::spawn_blocking(move || app.state::<Library>().store.view(&id).map_err(Failure::from))
        .await
        .map_err(|e| Failure::new("io", Some(e.to_string())))?
}
