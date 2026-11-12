//! `cargo run --release --example attempt_chat_probe -- <data folder> <dataset folder> [catalogue id]`:
//! the chat about a finished attempt, with the real model, on the catalogue question with the longest
//! base text. Imports the PDF from `<dataset>/exam_sum/` into a throwaway store (as the `#[ignore]`
//! catalogue test does), answers that question wrong, finishes, opens a chat scoped to the attempt and
//! asks why. The prompt itself is private to the engine, so the probe prints the base text's length
//! (4000 of it reach the model) and the reply's sentences. Jay-AI's files are only read.
use jay_study::Store;
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::catalog::ENTRIES;
use jay_study::chat::Scope;
use jay_study::exam::QuestionKind;
use jay_study::records::Mode;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (data, dataset) = (&args[1], &args[2]);
    let id = args.get(3).map_or("uepg-2025-cg-ingles", String::as_str);
    let entry = ENTRIES.iter().find(|e| e.id == id).expect("catalogue id");
    let root = std::env::temp_dir().join(format!("jay-s-attempt-chat-probe-{}", jay_study::id::new_id()));
    let store = Store::open(&root).expect("store");
    let cancel = AtomicBool::new(false);
    let part = root.join("catalog").join(format!("{id}.part"));
    std::fs::create_dir_all(part.parent().unwrap()).unwrap();
    std::fs::copy(format!("{dataset}/exam_sum/{}", entry.download.file), &part).unwrap();
    let exam = store.exam(&store.get_ready_made(id, &mut |_, _| {}, &cancel).expect("exam")).expect("exam");
    let (n, longest) = exam.questions.iter().enumerate().max_by_key(|(_, q)| q.passage.as_ref().map_or(0, |p| p.chars().count())).expect("questions");
    let base = longest.passage.as_ref().map_or(0, |p| p.chars().count());
    println!("exam {id}: {} questions; longest base text: question {} with {base} chars", exam.questions.len(), n + 1);
    let (a, _) = store.start_attempt(&exam.id, Mode::Exam, None).expect("attempt");
    let wrong = match longest.kind {
        QuestionKind::Choice => longest.options.iter().find(|o| Some(&o.label) != longest.answer.as_ref()).map(|o| o.label.clone()),
        QuestionKind::Sum => Some(if longest.answer.as_deref() == Some("01") { "02".to_owned() } else { "01".to_owned() }),
    };
    store.answer(&a.id, n, wrong, 20_000).expect("answer");
    store.finish_attempt(&a.id).expect("finish");
    let server = Server::start(&AiPaths::new(std::path::Path::new(data)), ModelChoice::Standard).expect("server");
    let question = format!("Por que errei a questão {}? O que o texto diz?", n + 1);
    // One new chat per seed (the server's fixed seed makes a run repeat).
    for seed in [1, 2, 3] {
        server.set_seed(Some(seed));
        let chat = store.new_chat(Scope { materials: vec![], attempt: Some(a.id.clone()) }).expect("chat");
        let t = Instant::now();
        match store.chat_ask(&chat.id, &question, &server, &cancel) {
            Ok(turn) => {
                println!("seed {seed}: {question}\n{:?} in {:.1}s, checks {:?}", turn.reply.kind, t.elapsed().as_secs_f32(), turn.reply.checks);
                for s in &turn.reply.sentences {
                    println!("- [{:?}] {}", s.status, s.text);
                }
            }
            Err(e) => println!("seed {seed}: ERROR after {:.1}s: {e:?}", t.elapsed().as_secs_f32()),
        }
    }
    drop(server);
    let _ = std::fs::remove_dir_all(root);
}
