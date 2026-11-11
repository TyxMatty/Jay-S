//! Jay-AI in the app (stage 2): download the runtime and a model, start the server when a request
//! comes, stop it 2 minutes after the last one (docs/ARQUITETURA.md, section 5). The page only sees
//! states and progress; the server's port and key stay here.

use crate::library::Failure;
use jay_study::ai::{AiError, AiPaths, ModelChoice, Server};
use jay_study::store::{read_json, write_json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

pub const STATUS_EVENT: &str = "ai-status";
pub const DOWNLOAD_EVENT: &str = "ai-download";
/// The server stops this long after its last request.
const IDLE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    Off,
    Loading,
    Ready,
    Working,
}

#[derive(Serialize, Deserialize)]
struct Prefs {
    model: ModelChoice,
}

pub struct Ai {
    paths: AiPaths,
    prefs_path: std::path::PathBuf,
    choice: Mutex<ModelChoice>,
    server: Mutex<Option<Server>>,
    state: Mutex<ServerState>,
    last_used: Mutex<Instant>,
    /// Held while a request runs: one at a time.
    busy: Mutex<()>,
    downloading: AtomicBool,
    cancel_download: Arc<AtomicBool>,
    cancel_request: Arc<AtomicBool>,
    /// How many times Stop was pressed: a request that waited for its turn sees one it missed.
    cancels: AtomicUsize,
}

impl Ai {
    pub fn new(data_root: &std::path::Path) -> Self {
        let prefs_path = data_root.join("ai.json");
        let choice = read_json::<Prefs>(&prefs_path).map(|p| p.model).unwrap_or(ModelChoice::Standard);
        Self {
            paths: AiPaths::new(data_root),
            prefs_path,
            choice: Mutex::new(choice),
            server: Mutex::new(None),
            state: Mutex::new(ServerState::Off),
            last_used: Mutex::new(Instant::now()),
            busy: Mutex::new(()),
            downloading: AtomicBool::new(false),
            cancel_download: Arc::new(AtomicBool::new(false)),
            cancel_request: Arc::new(AtomicBool::new(false)),
            cancels: AtomicUsize::new(0),
        }
    }

    fn set_state(&self, app: &AppHandle, state: ServerState) {
        *lock(&self.state) = state;
        let _ = app.emit(STATUS_EVENT, self.status());
    }

    pub fn status(&self) -> AiStatus {
        let model = |c| ModelStatus { ready: self.paths.model_ready(c), partial: self.paths.model_partial(c), size: jay_study::ai::model(c).size };
        AiStatus {
            runtime: self.paths.runtime_ready(),
            choice: *lock(&self.choice),
            standard: model(ModelChoice::Standard),
            light: model(ModelChoice::Light),
            server: *lock(&self.state),
            downloading: self.downloading.load(Ordering::Relaxed),
        }
    }

    /// Runs `f` with the server, starting it if needed. One request at a time; the idle timer starts
    /// over when it ends.
    pub fn with_server<T>(&self, app: &AppHandle, f: impl FnOnce(&Server, &AtomicBool) -> Result<T, AiError>) -> Result<T, AiError> {
        let cancels = self.cancels.load(Ordering::Relaxed);
        let _busy = lock(&self.busy);
        self.cancel_request.store(false, Ordering::Relaxed);
        // Stop pressed while this request waited behind another one is for it too: it used to be
        // forgotten here, and the request ran whole. (After the reset, so a Stop right now isn't
        // lost either: it either changed the count or set the flag after.)
        // shortcut: Stop is one for everything, the running request and the waiting ones; upgrade to
        // one flag per request if the screens ever need to stop only their own.
        if self.cancels.load(Ordering::Relaxed) != cancels {
            return Err(AiError::Cancelled);
        }
        let mut server = lock(&self.server);
        // A server that died (crash, killed from outside) is started again, instead of failing
        // every request until the idle watch lets go of it.
        if server.as_ref().is_some_and(|s| !s.healthy()) {
            server.take();
        }
        if server.is_none() {
            self.set_state(app, ServerState::Loading);
            // Read before the `match`: a lock taken in its head is held to its end, the whole load.
            // `status()` wants the same lock, so the window froze while the model loaded (measured:
            // 2 s, every command waiting) and a failed start locked itself in `set_state` below.
            let choice = *lock(&self.choice);
            match Server::start(&self.paths, choice) {
                Ok(s) => *server = Some(s),
                Err(e) => {
                    self.set_state(app, ServerState::Off);
                    return Err(e);
                }
            }
        }
        self.set_state(app, ServerState::Working);
        let result = f(server.as_ref().unwrap(), &self.cancel_request);
        *lock(&self.last_used) = Instant::now();
        self.set_state(app, ServerState::Ready);
        result
    }

    /// The server is loading or answering: an update now would cut the job.
    pub fn working(&self) -> bool {
        matches!(*lock(&self.state), ServerState::Loading | ServerState::Working)
    }

    /// The chosen model and the runtime are downloaded.
    pub fn ready(&self) -> bool {
        self.paths.runtime_ready() && self.paths.model_ready(*lock(&self.choice))
    }

    /// Stops a download and the reply in progress.
    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        self.cancels.fetch_add(1, Ordering::Relaxed);
        self.cancel_request.store(true, Ordering::Relaxed);
    }

    /// A fresh Stop flag for a download that is not the model's (`cancel` sets it).
    pub fn download_cancel(&self) -> Arc<AtomicBool> {
        self.cancel_download.store(false, Ordering::Relaxed);
        self.cancel_download.clone()
    }

    /// Stops the server (its memory comes back at once). A request in progress is cancelled first,
    /// and this waits for it to let go: call it off the main thread (`stop_off_main`), or the window
    /// freezes while a long job finishes (it did: "Liberar a memória" during an exam build).
    pub fn stop(&self, app: &AppHandle) {
        self.cancel_request.store(true, Ordering::Relaxed);
        let _busy = lock(&self.busy);
        lock(&self.server).take();
        self.set_state(app, ServerState::Off);
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Stops the server once it's been idle for `IDLE` (RNF-1: the model only stays in memory while it
/// works).
pub fn spawn_idle_watch(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(10));
            let ai = app.state::<Ai>();
            // A request in progress holds `busy`: never stop under it.
            // (A poisoned lock is free: skipping it would keep the server loaded forever.)
            let _busy = match ai.busy.try_lock() {
                Ok(guard) => guard,
                Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
                Err(std::sync::TryLockError::WouldBlock) => continue,
            };
            let idle = lock(&ai.last_used).elapsed() >= IDLE;
            if idle && lock(&ai.server).take().is_some() {
                *lock(&ai.state) = ServerState::Off;
                let _ = app.emit(STATUS_EVENT, ai.status());
            }
        }
    });
}

#[derive(Clone, Serialize)]
pub struct ModelStatus {
    ready: bool,
    /// Bytes of an unfinished download (it resumes from here).
    partial: u64,
    size: u64,
}

#[derive(Clone, Serialize)]
pub struct AiStatus {
    runtime: bool,
    choice: ModelChoice,
    standard: ModelStatus,
    light: ModelStatus,
    server: ServerState,
    downloading: bool,
}

impl From<AiError> for Failure {
    fn from(e: AiError) -> Self {
        let (code, detail) = match e {
            AiError::Cancelled => ("cancelled", None),
            AiError::Mismatch => ("mismatch", None),
            AiError::Network(d) => ("network", Some(d)),
            AiError::NotInstalled => ("not_installed", None),
            AiError::Start(d) => ("start", Some(d)),
            AiError::BadReply(d) => ("bad_reply", Some(d)),
            AiError::Io(d) => ("io", Some(d)),
        };
        Failure::new(code, detail)
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub(crate) enum DownloadEvent {
    Progress { what: &'static str, done: u64, total: u64 },
    Done,
    Error { error: Failure },
}

#[tauri::command]
pub fn ai_status(ai: State<Ai>) -> AiStatus {
    ai.status()
}

/// `Ai::stop` on a worker thread (Tauri runs sync commands on the main thread).
async fn stop_off_main(app: &AppHandle) {
    let app = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || app.state::<Ai>().stop(&app)).await;
}

#[tauri::command]
pub async fn ai_set_choice(app: AppHandle, choice: ModelChoice) -> Result<AiStatus, Failure> {
    let current = *lock(&app.state::<Ai>().choice);
    if current != choice {
        // The running server has the other model: stop it, the next request starts the new one.
        stop_off_main(&app).await;
        let ai = app.state::<Ai>();
        *lock(&ai.choice) = choice;
        write_json(&ai.prefs_path, &Prefs { model: choice }).map_err(|e| Failure::new("io", Some(e.to_string())))?;
    }
    Ok(app.state::<Ai>().status())
}

/// Downloads what's missing for the chosen model (runtime, then model), on a thread; progress and
/// the end come as `ai-download` events.
#[tauri::command]
pub fn ai_install(app: AppHandle, ai: State<Ai>) -> Result<(), Failure> {
    if ai.downloading.swap(true, Ordering::Relaxed) {
        return Err(Failure::new("busy", None));
    }
    ai.cancel_download.store(false, Ordering::Relaxed);
    let _ = app.emit(STATUS_EVENT, ai.status());
    let app = app.clone();
    std::thread::spawn(move || {
        let ai = app.state::<Ai>();
        let cancel = ai.cancel_download.clone();
        let choice = *lock(&ai.choice);
        let emit = |what| {
            let app = app.clone();
            move |done, total| {
                let _ = app.emit(DOWNLOAD_EVENT, DownloadEvent::Progress { what, done, total });
            }
        };
        let mut result = Ok(());
        if !ai.paths.runtime_ready() {
            result = ai.paths.install_runtime(&mut emit("runtime"), &cancel);
        }
        if result.is_ok() && !ai.paths.model_ready(choice) {
            result = ai.paths.install_model(choice, &mut emit("model"), &cancel);
        }
        ai.downloading.store(false, Ordering::Relaxed);
        let _ = app.emit(DOWNLOAD_EVENT, match result {
            Ok(()) => DownloadEvent::Done,
            Err(e) => DownloadEvent::Error { error: e.into() },
        });
        let _ = app.emit(STATUS_EVENT, ai.status());
        // Results finished before Jay-AI was downloaded go to it now.
        crate::study::analyze_pending(&app);
    });
    Ok(())
}

#[tauri::command]
pub fn ai_cancel(ai: State<Ai>) {
    ai.cancel();
}

#[tauri::command]
pub async fn ai_remove_model(app: AppHandle, choice: ModelChoice) -> Result<AiStatus, Failure> {
    if *lock(&app.state::<Ai>().choice) == choice {
        stop_off_main(&app).await;
    }
    let ai = app.state::<Ai>();
    ai.paths.remove_model(choice).map_err(|e| Failure::new("io", Some(e.to_string())))?;
    Ok(ai.status())
}

/// "Free the memory now": cancels what Jay-AI is doing and stops the server, off the main thread.
#[tauri::command]
pub async fn ai_stop(app: AppHandle) {
    stop_off_main(&app).await;
}

#[derive(Serialize)]
pub struct TestReply {
    reply: Value,
    seconds: f64,
    /// Seconds until the model was loaded (0 when it already was).
    load_seconds: f64,
}

/// A small question with a JSON schema: shows that Jay-AI answers, follows the format, and how
/// fast it is on this PC.
#[tauri::command]
pub async fn ai_test(app: AppHandle) -> Result<TestReply, Failure> {
    tauri::async_runtime::spawn_blocking(move || {
        let ai = app.state::<Ai>();
        let loaded = lock(&ai.server).is_some();
        let started = Instant::now();
        let mut load_seconds = 0.0;
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "capital": { "type": "string" }, "curiosidade": { "type": "string" } },
            "required": ["capital", "curiosidade"],
        });
        let messages = [
            serde_json::json!({ "role": "system", "content": "Você é o Jay-AI, um tutor de estudos. Responda em português do Brasil, em JSON." }),
            serde_json::json!({ "role": "user", "content": "Qual é a capital do Paraná? Dê também uma curiosidade curta sobre ela." }),
        ];
        let reply = ai.with_server(&app, |server, cancel| {
            if !loaded {
                load_seconds = started.elapsed().as_secs_f64();
            }
            server.chat(&messages, Some(&schema), 200, &mut |_| {}, cancel)
        })?;
        Ok(TestReply { reply: jay_study::ai::json_in(&reply)?, seconds: started.elapsed().as_secs_f64() - load_seconds, load_seconds })
    })
    .await
    .map_err(|e| Failure::new("io", Some(e.to_string())))?
    .map_err(|e: AiError| e.into())
}
