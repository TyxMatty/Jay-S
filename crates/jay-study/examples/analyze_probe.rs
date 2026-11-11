//! `cargo run --release --example analyze_probe -- <data folder> <uepg-2025-conhecimentos-gerais.pdf>`:
//! the analysis of a 60-question UEPG attempt with the real model. Builds the catalogue exam in a
//! throwaway store, then analyzes (1) an attempt with one answer and the rest blank, (2) an attempt
//! with every question answered wrong; prints the prompt size, the time and the analysis (or the
//! error). Jay-AI's runtime and model are only read from the data folder.
use jay_study::Store;
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::exam::QuestionKind;
use jay_study::records::Mode;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = std::env::temp_dir().join(format!("jay-s-analyze-probe-{}", jay_study::id::new_id()));
    let store = Store::open(&root).expect("store");
    let cancel = AtomicBool::new(false);
    let part = root.join("catalog").join("uepg-2025-cg-ingles.part");
    std::fs::create_dir_all(part.parent().unwrap()).unwrap();
    std::fs::copy(&args[2], &part).unwrap();
    let exam = store.exam(&store.get_ready_made("uepg-2025-cg-ingles", &mut |_, _| {}, &cancel).expect("exam")).expect("exam");
    println!("exam: {} questions", exam.questions.len());
    let server = Server::start(&AiPaths::new(std::path::Path::new(&args[1])), ModelChoice::Standard).expect("server");
    for (name, all_wrong) in [("one answered, 59 blank", false), ("all 60 answered wrong", true)] {
        let (a, _) = store.start_attempt(&exam.id, Mode::Exam, None).expect("attempt");
        for (i, q) in exam.questions.iter().enumerate().filter(|(i, _)| all_wrong || *i == 0) {
            if q.annulled {
                continue;
            }
            let wrong = match q.kind {
                QuestionKind::Choice => q.options.iter().find(|o| Some(&o.label) != q.answer.as_ref()).map(|o| o.label.clone()),
                QuestionKind::Sum => Some(if q.answer.as_deref() == Some("01") { "02".to_owned() } else { "01".to_owned() }),
            };
            store.answer(&a.id, i, wrong, 20_000).expect("answer");
        }
        let a = store.finish_attempt(&a.id).expect("finish");
        let (messages, _) = store.analysis_request(&a).expect("request");
        let chars: usize = messages.iter().map(|m| m["content"].as_str().unwrap_or("").chars().count()).sum();
        println!("\n=== {name}: prompt {chars} chars");
        let t = Instant::now();
        match store.analyze(&a.id, &server, &cancel) {
            Ok(done) => {
                let an = done.analysis.unwrap();
                println!("analysis in {:.1}s: {}", t.elapsed().as_secs_f32(), serde_json::to_string_pretty(&an).unwrap().lines().take(14).collect::<Vec<_>>().join("\n"));
            }
            Err(e) => println!("ERROR after {:.1}s: {e:?}", t.elapsed().as_secs_f32()),
        }
    }
    drop(server);
    let _ = std::fs::remove_dir_all(root);
}
