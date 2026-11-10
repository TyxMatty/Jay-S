//! `cargo run --release --example order_probe -- <data folder> <section>… [exam.txt key.txt]`: what
//! the order of the schema's fields does to each feature, with the real model. Run it on two builds
//! (or two orders) and compare. Sections: `solve` (needs the text of an exam and of its key: how many
//! answers match the official key), `gen` (questions the solver agrees with), `check` (verdicts on
//! answers whose verdict is known), `explain`, `chat`, `analysis` (printed, to be read).
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::chat::{Scope, Status};
use jay_study::explain::Level;
use jay_study::records::{Exam, Mode};
use jay_study::{Format, Store, exam, generate};
use std::sync::atomic::AtomicBool;

const CLEAN: &str = "# Fotossíntese\n\nA fotossíntese é o processo em que plantas, algas e algumas bactérias produzem glicose a partir de gás carbônico e água, usando a energia da luz. Nas plantas, ela acontece nos cloroplastos, organelas que têm clorofila, o pigmento verde que absorve a luz.\n\n## Fase clara\n\nAcontece nas membranas dos tilacoides. A luz excita a clorofila, a água é quebrada (fotólise) e libera oxigênio. A energia vira ATP e NADPH.\n\n## Fase escura\n\nAcontece no estroma do cloroplasto. No ciclo de Calvin, a enzima rubisco fixa o CO2 e, com o ATP e o NADPH da fase clara, forma glicose.\n\n## Fatores\n\nA intensidade da luz, a concentração de CO2 e a temperatura limitam a velocidade da fotossíntese.\n";

/// (question, the student's answer, the verdict a teacher would give)
const ANSWERS: [(&str, &str, &str); 10] = [
    ("Onde acontece a fase clara?", "Nas membranas dos tilacoides.", "right"),
    ("Onde acontece a fase clara?", "No estroma do cloroplasto.", "wrong"),
    ("O que a fase clara produz?", "Só ATP.", "partial"),
    ("Qual enzima fixa o CO2?", "A rubisco, no ciclo de Calvin.", "right"),
    ("Qual enzima fixa o CO2?", "A clorofila.", "wrong"),
    ("Quais fatores limitam a velocidade da fotossíntese?", "A luz e a temperatura.", "partial"),
    ("Quais fatores limitam a velocidade da fotossíntese?", "A intensidade da luz, a concentração de CO2 e a temperatura.", "right"),
    ("De onde vem o oxigênio liberado na fotossíntese?", "Do gás carbônico.", "wrong"),
    ("De onde vem o oxigênio liberado na fotossíntese?", "Da quebra da água, a fotólise.", "right"),
    ("Em que organela acontece a fotossíntese nas plantas?", "Nas mitocôndrias.", "wrong"),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let on = |s: &str| args.iter().skip(2).any(|a| a == s);
    let files: Vec<&String> = args.iter().skip(2).filter(|a| a.ends_with(".txt")).collect();
    let root = std::env::temp_dir().join(format!("jay-s-order-{}", jay_study::id::new_id()));
    let store = Store::open(&root).expect("store");
    let clean = store.import_bytes("fotossintese.md", Format::Text, CLEAN.as_bytes().to_vec()).expect("clean").id;
    let server = Server::start(&AiPaths::new(std::path::Path::new(&args[1])), ModelChoice::Standard).expect("server");
    let cancel = AtomicBool::new(false);
    let t = std::time::Instant::now();

    if on("solve") {
        let mut questions = exam::split_questions(&std::fs::read_to_string(files[0]).expect("exam"));
        exam::apply_answer_key(&mut questions, &exam::parse_answer_key(&std::fs::read_to_string(files[1]).expect("key")).expect("key"));
        // From 6 on: 1 to 5 exist twice (English and Spanish).
        let (mut right, mut failed, mut n) = (0, 0, 0);
        for q in questions.iter().filter(|q| q.number > 5 && q.answer.is_some() && q.options.len() == 5).take(90) {
            n += 1;
            match generate::solve(&server, q, &cancel) {
                Ok((a, _)) => right += (Some(&a) == q.answer.as_ref()) as usize,
                Err(_) => failed += 1,
            }
        }
        println!("== solve: {right} of {n} match the official key ({failed} failed) in {:?}", t.elapsed());
    }

    if on("gen") {
        for id in ["enem", "uepg"] {
            let style = exam::style(id).expect("style");
            let (mut kept, mut answers) = (0, std::collections::BTreeMap::<String, usize>::new());
            for i in 0..8 {
                let focuses = generate::focuses(CLEAN);
                // One try: the share the solver agrees with is the measure.
                if let Ok(Some(q)) = generate::new_question(&server, &style, CLEAN, focuses.get(i % focuses.len().max(1)).map(String::as_str), &[], i as u32 + 1, "probe", 1, &cancel) {
                    kept += 1;
                    *answers.entry(q.answer.clone().unwrap()).or_default() += 1;
                    println!("{id} {}: {} [{}] {}", i + 1, q.stem.chars().take(140).collect::<String>(), q.answer.unwrap(), q.explanation.unwrap_or_default().chars().take(200).collect::<String>());
                }
            }
            println!("== gen {id}: kept {kept} of 8 at the first try; answers {answers:?} ({:?})", t.elapsed());
        }
    }

    if on("check") {
        let (mut right, mut follow_ok, mut n) = (0, 0, 0);
        for _ in 0..2 {
            for (question, answer, want) in ANSWERS {
                let c = store.check_answer(&clean, question, answer, "português", &server, &cancel).expect("check");
                n += 1;
                right += (c.verdict == want) as usize;
                follow_ok += (c.follow_up.is_none() == (c.verdict == "right")) as usize;
                println!("check [{want} -> {}] {answer} | {} | follow: {:?}", c.verdict, c.feedback.chars().take(140).collect::<String>(), c.follow_up);
            }
        }
        println!("== check: {right} of {n} verdicts as expected; follow-up consistent with the verdict in {follow_ok} of {n} ({:?})", t.elapsed());
    }

    if on("explain") {
        for i in 0..3 {
            let e = store.explain(&clean, "português", Level::Balanced, None, &server, &mut |_, _| {}, &cancel).expect("explain");
            println!("-- explain {}: {} | {} paragraphs, {} points", i + 1, e.title, e.summary.len(), e.key_points.len());
            for c in &e.concepts {
                println!("  [{}] {} || ex.: {}", c.term, c.explanation, c.example);
            }
            for q in &e.questions {
                println!("  ? {} || dica: {}", q.question, q.hint);
            }
        }
        println!("== explain done ({:?})", t.elapsed());
    }

    if on("chat") {
        let (mut confirmed, mut unconfirmed, mut rejected, mut retried) = (0, 0, 0, 0);
        for question in ["Onde acontece a fase clara?", "O que a rubisco faz?", "De onde vem o oxigênio?", "O que limita a velocidade da fotossíntese?", "O que a fase escura usa da fase clara?", "O que é a clorofila?"] {
            let chat = store.new_chat(Scope { materials: vec![clean.clone()], attempt: None }).expect("chat");
            let r = store.chat_ask(&chat.id, question, &server, &cancel).expect("ask").reply;
            confirmed += r.sentences.iter().filter(|s| s.status == Status::Confirmed).count();
            unconfirmed += r.sentences.iter().filter(|s| s.status == Status::Unconfirmed).count();
            rejected += r.checks.rejected_quotes;
            retried += r.checks.retried as usize;
            println!("chat {:?} {question} | {}", r.kind, r.sentences.iter().map(|s| format!("{} ({:?})", s.text, s.status)).collect::<Vec<_>>().join(" "));
        }
        println!("== chat: {confirmed} sentences confirmed, {unconfirmed} unconfirmed, {rejected} quotes rejected, {retried} of 6 retried ({:?})", t.elapsed());
    }

    if on("analysis") {
        let exam: Exam = serde_json::from_value(serde_json::json!({
            "id": jay_study::id::new_id(), "title": "Fotossíntese", "style": "enem", "created": 0, "materials": [clean],
            "questions": [
                { "number": 1, "kind": "choice", "stem": "Em que organela acontece a fotossíntese nas plantas?", "options": [{ "label": "A", "text": "Cloroplasto" }, { "label": "B", "text": "Mitocôndria" }, { "label": "C", "text": "Ribossomo" }], "answer": "A", "topic": "Biologia: organelas", "explanation": "Nos cloroplastos, onde está a clorofila." },
                { "number": 2, "kind": "choice", "stem": "Onde acontece o ciclo de Calvin?", "options": [{ "label": "A", "text": "Nos tilacoides" }, { "label": "B", "text": "No estroma" }, { "label": "C", "text": "No núcleo" }], "answer": "B", "topic": "Biologia: fase escura", "explanation": "A fase escura acontece no estroma do cloroplasto." },
                { "number": 3, "kind": "choice", "stem": "De onde vem o oxigênio liberado?", "options": [{ "label": "A", "text": "Do CO2" }, { "label": "B", "text": "Da glicose" }, { "label": "C", "text": "Da água" }], "answer": "C", "topic": "Biologia: fase clara", "explanation": "Da fotólise da água, na fase clara." },
            ],
        }))
        .expect("exam");
        store.save_exam(&exam).expect("save");
        for i in 0..3 {
            let (a, _) = store.start_attempt(&exam.id, Mode::Exam, None).expect("attempt");
            for (q, marked) in ["A", "A", "A"].iter().enumerate() {
                store.answer(&a.id, q, Some(marked.to_string()), 5000).expect("answer");
            }
            store.finish_attempt(&a.id).expect("finish");
            let a = store.analyze(&a.id, &server, &cancel).expect("analyze");
            println!("-- analysis {}: {}", i + 1, serde_json::to_string_pretty(&a.analysis).unwrap());
        }
        println!("== analysis done ({:?})", t.elapsed());
    }
    std::fs::remove_dir_all(&root).ok();
}
