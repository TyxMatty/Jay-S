//! Exams, attempts and flashcards (stages 3–6). Long work (building an exam or a deck, analyzing a
//! result) runs one at a time on a thread, with progress as `study-job` events; Jay-AI is started for
//! it through `Ai::with_server`. Answers reach the page only once graded (docs/ARQUITETURA.md,
//! section 2, "Correção"); a finished attempt goes straight to Jay-AI for analysis.

use crate::ai::Ai;
use crate::library::{Failure, Library};
use jay_study::ai::AiError;
use jay_study::cards::{DeckSummary, DueCard};
use jay_study::exam::{self, Style};
use jay_study::records::{AnswerError, Attempt, BuildError, BuildPlan, BuildStep, ExamSummary, Graded, Mode, OpenQuestion, Profile};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, State};

pub const JOB_EVENT: &str = "study-job";

/// The one long job running, if any.
#[derive(Default)]
pub struct Jobs {
    running: AtomicBool,
}

impl From<BuildError> for Failure {
    fn from(e: BuildError) -> Self {
        match e {
            BuildError::Store(e) => e.into(),
            BuildError::Ai(e) => e.into(),
            BuildError::Empty => Failure::new("empty", None),
            BuildError::UnknownStyle => Failure::new("unknown_style", None),
        }
    }
}

impl From<AnswerError> for Failure {
    fn from(e: AnswerError) -> Self {
        match e {
            AnswerError::Store(e) => e.into(),
            AnswerError::Finished => Failure::new("finished", None),
            AnswerError::TimeUp => Failure::new("time_up", None),
            AnswerError::NoQuestion => Failure::new("not_found", None),
            AnswerError::Locked => Failure::new("locked", None),
            AnswerError::Invalid => Failure::new("invalid_answer", None),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
enum JobEvent {
    Progress { kind: &'static str, step: BuildStep },
    Done { kind: &'static str, id: String },
    Error { kind: &'static str, id: Option<String>, error: Failure },
}

/// Runs `work` on a thread as the one job; `Err(busy)` when another is running.
fn spawn_job(app: &AppHandle, work: impl FnOnce(&AppHandle) + Send + 'static) -> Result<(), Failure> {
    if app.state::<Jobs>().running.swap(true, Ordering::Relaxed) {
        return Err(Failure::new("busy", None));
    }
    let app = app.clone();
    std::thread::spawn(move || {
        work(&app);
        app.state::<Jobs>().running.store(false, Ordering::Relaxed);
    });
    Ok(())
}

#[derive(Serialize)]
pub struct StyleInfo {
    id: String,
    name: String,
    kind: exam::QuestionKind,
    options: u32,
    description: String,
}

#[tauri::command]
pub fn styles_list() -> Vec<StyleInfo> {
    exam::styles().into_iter().map(|s: Style| StyleInfo { id: s.id, name: s.name, kind: s.kind, options: s.options, description: s.description }).collect()
}

#[derive(Serialize)]
pub struct ExamMaterialInfo {
    questions: usize,
    sum: usize,
    key: Option<String>,
    answered: usize,
}

/// What an exam material holds, for the builder: how many questions, and the key that fits it.
#[tauri::command]
pub fn exam_material_info(library: State<Library>, id: String) -> Result<ExamMaterialInfo, Failure> {
    let key = library.store.best_key(&id);
    let qs = library.store.material_questions(&id, key.as_deref())?;
    Ok(ExamMaterialInfo { questions: qs.len(), sum: qs.iter().filter(|q| q.kind == exam::QuestionKind::Sum).count(), answered: qs.iter().filter(|q| q.answer.is_some()).count(), key })
}

#[tauri::command]
pub fn exam_build(app: AppHandle, plan: BuildPlan) -> Result<(), Failure> {
    spawn_job(&app, move |app| {
        let library = app.state::<Library>();
        let ai = app.state::<Ai>();
        let emit = |step| {
            let _ = app.emit(JOB_EVENT, JobEvent::Progress { kind: "exam", step });
        };
        let mut progress = emit;
        // Jay-AI when it's downloaded (new questions, answers for questions without a key, topics);
        // without it, only reused questions with a key.
        let result = if ai.ready() {
            ai.with_server(app, |server, cancel| library.store.build_exam(&plan, Some(server), &mut progress, cancel).map_err(|e| match e {
                BuildError::Ai(e) => e,
                other => AiError::Io(format!("{other:?}")),
            }))
            .map_err(Failure::from)
        } else {
            library.store.build_exam(&plan, None, &mut progress, &AtomicBool::new(false)).map_err(Failure::from)
        };
        let _ = app.emit(JOB_EVENT, match result {
            Ok(exam) => JobEvent::Done { kind: "exam", id: exam.id },
            Err(error) => JobEvent::Error { kind: "exam", id: None, error },
        });
    })
}

#[tauri::command]
pub fn exams_list(library: State<Library>) -> Vec<ExamSummary> {
    library.store.exams()
}

#[tauri::command]
pub fn exam_delete(library: State<Library>, id: String) -> Result<(), Failure> {
    let _writing = library.writing();
    Ok(library.store.delete_exam(&id)?)
}

#[derive(Serialize)]
pub struct Started {
    attempt: Attempt,
    questions: Vec<OpenQuestion>,
    /// Learning mode: the questions already answered, graded (when resuming).
    graded: Vec<Option<Graded>>,
}

#[tauri::command]
pub fn attempt_start(library: State<Library>, exam: String, mode: Mode, minutes: Option<u64>) -> Result<Started, Failure> {
    let _writing = library.writing();
    let (attempt, questions) = library.store.start_attempt(&exam, mode, minutes.map(|m| m * 60))?;
    let graded = vec![None; questions.len()];
    Ok(Started { attempt, questions, graded })
}

/// An unfinished attempt, to go on after closing the app.
#[tauri::command]
pub fn attempt_resume(library: State<Library>, id: String) -> Result<Started, Failure> {
    let attempt = library.store.attempt(&id)?;
    if attempt.finished.is_some() {
        return Err(Failure::new("finished", None));
    }
    let exam = library.store.exam(&attempt.exam)?;
    let questions: Vec<OpenQuestion> = exam.questions.iter().map(OpenQuestion::from).collect();
    // Only learning mode shows what was already graded; in exam mode nothing is revealed.
    let graded = exam
        .questions
        .iter()
        .zip(&attempt.answers)
        .map(|(q, a)| (attempt.mode == Mode::Learn && a.is_some()).then(|| jay_study::records::grade_one(q, a, &exam.style)))
        .collect();
    Ok(Started { attempt, questions, graded })
}

#[tauri::command]
pub fn attempt_answer(library: State<Library>, id: String, index: usize, marked: Option<String>, ms: u64) -> Result<Option<Graded>, Failure> {
    let _writing = library.writing();
    Ok(library.store.answer(&id, index, marked, ms)?)
}

/// Ends the attempt, returns the result, and sends it to Jay-AI right away.
#[tauri::command]
pub fn attempt_finish(app: AppHandle, library: State<Library>, id: String) -> Result<Attempt, Failure> {
    let attempt = {
        let _writing = library.writing();
        library.store.finish_attempt(&id)?
    };
    analyze_pending(&app);
    Ok(attempt)
}

/// Every finished attempt Jay-AI hasn't analyzed yet (one just finished, or ones from before Jay-AI
/// was downloaded) goes to it, one after the other, when it's ready and nothing else runs.
pub fn analyze_pending(app: &AppHandle) {
    if !app.state::<Ai>().ready() {
        return;
    }
    let _ = spawn_job(app, |app| {
        let library = app.state::<Library>();
        let ai = app.state::<Ai>();
        let pending: Vec<String> = library.store.attempts().into_iter().filter(|a| a.result.is_some() && a.analysis.is_none()).map(|a| a.id).collect();
        for id in pending {
            let _ = app.emit(JOB_EVENT, JobEvent::Progress { kind: "analysis", step: BuildStep::Reading });
            let result = ai.with_server(app, |server, cancel| library.store.analyze(&id, server, cancel).map_err(|e| match e {
                BuildError::Ai(e) => e,
                other => AiError::Io(format!("{other:?}")),
            }));
            let _ = app.emit(JOB_EVENT, match result {
                Ok(_) => JobEvent::Done { kind: "analysis", id },
                Err(e) => JobEvent::Error { kind: "analysis", id: Some(id), error: e.into() },
            });
        }
    });
}

#[tauri::command]
pub fn attempt_analyze(app: AppHandle) {
    analyze_pending(&app);
}

#[derive(Serialize)]
pub struct AttemptSummary {
    id: String,
    exam: String,
    title: String,
    mode: Mode,
    started: u64,
    finished: Option<u64>,
    points: Option<f32>,
    total: usize,
    answered: usize,
    analyzed: bool,
    time_limit: Option<u64>,
}

#[tauri::command]
pub fn attempts_list(library: State<Library>) -> Vec<AttemptSummary> {
    library
        .store
        .attempts()
        .into_iter()
        .map(|a| AttemptSummary {
            points: a.result.as_ref().map(|r| r.points),
            total: a.answers.len(),
            answered: a.answers.iter().filter(|x| x.is_some()).count(),
            analyzed: a.analysis.is_some(),
            id: a.id,
            exam: a.exam,
            title: a.title,
            mode: a.mode,
            started: a.started,
            finished: a.finished,
            time_limit: a.time_limit,
        })
        .collect()
}

/// A finished attempt with its result (answers, explanations, analysis), and the exam's questions.
#[tauri::command]
pub fn attempt_result(library: State<Library>, id: String) -> Result<(Attempt, Vec<OpenQuestion>), Failure> {
    let attempt = library.store.attempt(&id)?;
    if attempt.finished.is_none() {
        return Err(Failure::new("not_finished", None));
    }
    let questions = library.store.exam(&attempt.exam).map(|e| e.questions.iter().map(OpenQuestion::from).collect()).unwrap_or_default();
    Ok((attempt, questions))
}

#[tauri::command]
pub fn attempt_delete(library: State<Library>, id: String) -> Result<(), Failure> {
    let _writing = library.writing();
    Ok(library.store.delete_attempt(&id)?)
}

#[tauri::command]
pub fn profile_get(library: State<Library>) -> Profile {
    library.store.profile()
}

#[tauri::command]
pub fn decks_list(library: State<Library>) -> Vec<DeckSummary> {
    library.store.decks()
}

#[tauri::command]
pub fn deck_build(app: AppHandle, title: String, materials: Vec<String>, count: usize) -> Result<(), Failure> {
    if !app.state::<Ai>().ready() {
        return Err(AiError::NotInstalled.into());
    }
    spawn_job(&app, move |app| {
        let library = app.state::<Library>();
        let ai = app.state::<Ai>();
        let result = ai.with_server(app, |server, cancel| {
            let mut progress = |done, total| {
                let _ = app.emit(JOB_EVENT, JobEvent::Progress { kind: "deck", step: BuildStep::Writing { done, total } });
            };
            library.store.build_deck(&title, &materials, count, server, &mut progress, cancel).map_err(|e| match e {
                BuildError::Ai(e) => e,
                other => AiError::Io(format!("{other:?}")),
            })
        });
        let _ = app.emit(JOB_EVENT, match result {
            Ok(deck) => JobEvent::Done { kind: "deck", id: deck.id },
            Err(e) => JobEvent::Error { kind: "deck", id: None, error: e.into() },
        });
    })
}

#[tauri::command]
pub fn deck_due(library: State<Library>, id: String) -> Result<Vec<DueCard>, Failure> {
    Ok(library.store.due_cards(&id, 50)?)
}

#[tauri::command]
pub fn card_review(library: State<Library>, deck: String, index: usize, quality: u8) -> Result<(), Failure> {
    let _writing = library.writing();
    if !matches!(quality, 1 | 3 | 4 | 5) {
        return Err(Failure::new("invalid_answer", None));
    }
    Ok(library.store.review_card(&deck, index, quality)?)
}

#[tauri::command]
pub fn card_vote(library: State<Library>, too_long: bool) -> Result<Profile, Failure> {
    let _writing = library.writing();
    Ok(library.store.card_vote(too_long)?)
}

#[tauri::command]
pub fn deck_delete(library: State<Library>, id: String) -> Result<(), Failure> {
    let _writing = library.writing();
    Ok(library.store.delete_deck(&id)?)
}

/// Stops the running job (and Jay-AI's reply in progress).
#[tauri::command]
pub fn job_cancel(ai: State<Ai>) {
    ai.cancel();
}

#[tauri::command]
pub fn explain_get(library: State<Library>, id: String) -> Result<Option<jay_study::explain::Explanation>, Failure> {
    Ok(library.store.explanation(&id)?)
}

/// Summarizes and explains a material (a job: progress over the parts read, then `done`).
#[tauri::command]
pub fn explain_build(app: AppHandle, id: String, language: String, level: jay_study::explain::Level, focus: Option<String>) -> Result<(), Failure> {
    if !app.state::<Ai>().ready() {
        return Err(AiError::NotInstalled.into());
    }
    if jay_study::explain::clean_language(&language).is_none() {
        return Err(Failure::new("bad_language", None));
    }
    spawn_job(&app, move |app| {
        let library = app.state::<Library>();
        let ai = app.state::<Ai>();
        let result = ai.with_server(app, |server, cancel| {
            let mut progress = |done, total| {
                let _ = app.emit(JOB_EVENT, JobEvent::Progress { kind: "explain", step: BuildStep::Writing { done, total } });
            };
            library.store.explain(&id, &language, level, focus.as_deref(), server, &mut progress, cancel).map_err(|e| match e {
                BuildError::Ai(e) => e,
                other => AiError::Io(format!("{other:?}")),
            })
        });
        let _ = app.emit(JOB_EVENT, match result {
            Ok(_) => JobEvent::Done { kind: "explain", id },
            Err(e) => JobEvent::Error { kind: "explain", id: Some(id), error: e.into() },
        });
    })
}

/// Runs one quick Jay-AI request off the main thread (it waits its turn behind a running job).
async fn quick<T: Send + 'static>(app: AppHandle, work: impl FnOnce(&Library, &jay_study::ai::Server, &AtomicBool) -> Result<T, BuildError> + Send + 'static) -> Result<T, Failure> {
    if !app.state::<Ai>().ready() {
        return Err(AiError::NotInstalled.into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let library = app.state::<Library>();
        let ai = app.state::<Ai>();
        let mut out = None;
        let result = ai.with_server(&app, |server, cancel| {
            out = Some(work(&library, server, cancel));
            Ok(())
        });
        match (result, out) {
            (Err(e), _) => Err(e.into()),
            (Ok(()), Some(r)) => r.map_err(Failure::from),
            (Ok(()), None) => Err(Failure::new("io", None)),
        }
    })
    .await
    .map_err(|e| Failure::new("io", Some(e.to_string())))?
}

#[tauri::command]
pub async fn explain_check(app: AppHandle, id: String, question: String, answer: String, language: String) -> Result<jay_study::explain::Check, Failure> {
    quick(app, move |library, server, cancel| library.store.check_answer(&id, &question, &answer, &language, server, cancel)).await
}

#[tauri::command]
pub async fn explain_ask(app: AppHandle, id: String, question: String, language: String) -> Result<String, Failure> {
    quick(app, move |library, server, cancel| library.store.ask(&id, &question, &language, server, cancel)).await
}
