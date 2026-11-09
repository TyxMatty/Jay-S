//! Materials (stage 1): attaching through the dialog or a drop on the window, listing, changing the
//! kind, reading the text, deleting. The page never names a path: the dialog and the drop are handled
//! here, and the page only gets ids back (docs/ARQUITETURA.md, section 7).

use jay_study::store::StoreError;
use jay_study::{Material, MaterialKind, Store};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Events the page listens to while files are read (one per file).
pub const IMPORT_EVENT: &str = "materials-import";
/// How much text "Show text" brings to the page.
const PREVIEW_CHARS: usize = 20_000;

/// What the student chose when attaching: the kind (from the Materials tab they're on) and, for an
/// answer key, the exam it belongs to.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Attach {
    #[serde(default)]
    pub kind: Option<MaterialKind>,
    #[serde(default)]
    pub exam: Option<String>,
}

pub struct Library {
    pub store: Store,
    /// One writer at a time: imports run in order, and a change never races a delete
    /// (docs/ARQUITETURA.md, section 7, item 7).
    write: Mutex<()>,
    /// Where a file dropped on the window goes (the Materials tab open, as the page tells).
    pub drop_target: Mutex<Attach>,
}

impl Library {
    pub fn new(store: Store) -> Self {
        Self { store, write: Mutex::new(()), drop_target: Mutex::new(Attach::default()) }
    }

    /// Held by every command that changes a file: Tauri runs commands in parallel, and two quick
    /// answers would otherwise read and write the same attempt at once and lose one.
    pub fn writing(&self) -> std::sync::MutexGuard<'_, ()> {
        self.write.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// An error the page can translate: a code and, for some, a detail.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    code: &'static str,
    detail: Option<String>,
}

impl Failure {
    pub fn new(code: &'static str, detail: Option<String>) -> Self {
        Self { code, detail }
    }
}

impl From<StoreError> for Failure {
    fn from(e: StoreError) -> Self {
        use jay_study::ExtractError as E;
        match e {
            StoreError::NotFound => Self { code: "not_found", detail: None },
            StoreError::Extract(E::TooBig) => Self { code: "too_big", detail: None },
            StoreError::Extract(E::Unsupported) => Self { code: "unsupported", detail: None },
            StoreError::Extract(E::Invalid(d)) => Self { code: "invalid", detail: Some(d) },
            StoreError::Extract(E::Io(d)) => Self { code: "io", detail: Some(d) },
            StoreError::Io(e) => Self { code: "io", detail: Some(e.to_string()) },
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
enum ImportEvent {
    /// `token` ties the later event to this card.
    Reading { token: u64, name: String },
    Done { token: u64, material: Material },
    Error { token: u64, name: String, error: Failure },
}

/// Reads `paths` one by one on a thread of its own, telling the page as each starts and ends.
pub fn import(app: &AppHandle, paths: Vec<PathBuf>, attach: Attach) {
    if paths.is_empty() {
        return;
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let jobs: Vec<(u64, PathBuf, String)> = paths
        .into_iter()
        .map(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            (NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed), p, name)
        })
        .collect();
    for (token, _, name) in &jobs {
        let _ = app.emit(IMPORT_EVENT, ImportEvent::Reading { token: *token, name: name.clone() });
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let library = app.state::<Library>();
        for (token, path, name) in jobs {
            let result = {
                let _guard = library.write.lock().unwrap_or_else(|e| e.into_inner());
                // The student's choice wins over what the rules detected; an answer key attached
                // for an exam becomes that exam's key.
                library.store.import(&path).and_then(|m| match attach.kind {
                    Some(kind) if kind != m.kind => library.store.set_kind(&m.id, kind),
                    _ => Ok(m),
                }).and_then(|m| {
                    if let (Some(exam), MaterialKind::Key) = (&attach.exam, m.kind) {
                        library.store.set_key(exam, jay_study::KeyChoice::Material(m.id.clone()))?;
                    }
                    Ok(m)
                })
            };
            let event = match result {
                Ok(material) => ImportEvent::Done { token, material },
                Err(e) => ImportEvent::Error { token, name, error: e.into() },
            };
            let _ = app.emit(IMPORT_EVENT, event);
        }
    });
}

/// The files in a command line ("Open with", a second launch): everything after the program that
/// isn't a `--flag`, relative to `cwd`.
pub fn paths_from_args(args: &[String], cwd: &std::path::Path) -> Vec<PathBuf> {
    args.iter().skip(1).filter(|a| !a.starts_with("--")).map(|a| cwd.join(a)).collect()
}

/// Files from the first launch's command line, attached once the page listens (its first list).
#[derive(Default)]
pub struct StartupFiles(pub Mutex<Vec<PathBuf>>);

#[tauri::command]
pub fn materials_list(app: AppHandle, library: State<Library>, startup: State<StartupFiles>) -> Vec<Material> {
    import(&app, std::mem::take(&mut *startup.0.lock().unwrap_or_else(|e| e.into_inner())), Attach::default());
    library.store.materials()
}

#[tauri::command]
pub async fn materials_pick(app: AppHandle, attach: Option<Attach>) {
    // async: the blocking dialog runs off the main thread.
    let picked = app
        .dialog()
        .file()
        .add_filter("PDF, DOCX, text, photos", &["pdf", "docx", "txt", "md", "markdown", "png", "jpg", "jpeg"])
        .add_filter("*", &["*"])
        .blocking_pick_files()
        .unwrap_or_default();
    import(&app, picked.into_iter().filter_map(|p| p.into_path().ok()).collect(), attach.unwrap_or_default());
}

/// The Materials tab the page shows: files dropped on the window take its kind.
#[tauri::command]
pub fn materials_drop_target(library: State<Library>, attach: Attach) {
    *library.drop_target.lock().unwrap_or_else(|e| e.into_inner()) = attach;
}

/// Which answer key goes with an exam (automatic, none, or a key material).
#[tauri::command]
pub fn materials_set_key(library: State<Library>, id: String, key: jay_study::KeyChoice) -> Result<Material, Failure> {
    let _guard = library.write.lock().unwrap_or_else(|e| e.into_inner());
    Ok(library.store.set_key(&id, key)?)
}

#[derive(Serialize)]
pub struct KeyInfo {
    /// The key in use (the choice, or the one found automatically).
    key: Option<String>,
    choice: jay_study::KeyChoice,
}

/// The key an exam uses now.
#[tauri::command]
pub fn materials_key(library: State<Library>, id: String) -> Result<KeyInfo, Failure> {
    let m = library.store.material(&id)?;
    Ok(KeyInfo { key: library.store.key_for(&id), choice: m.key })
}

#[tauri::command]
pub fn materials_set_kind(library: State<Library>, id: String, kind: MaterialKind) -> Result<Material, Failure> {
    let _guard = library.write.lock().unwrap_or_else(|e| e.into_inner());
    Ok(library.store.set_kind(&id, kind)?)
}

#[tauri::command]
pub fn materials_text(library: State<Library>, id: String) -> Result<String, Failure> {
    Ok(library.store.text(&id, PREVIEW_CHARS)?)
}

#[tauri::command]
pub fn materials_delete(library: State<Library>, id: String) -> Result<(), Failure> {
    let _guard = library.write.lock().unwrap_or_else(|e| e.into_inner());
    Ok(library.store.delete(&id)?)
}
