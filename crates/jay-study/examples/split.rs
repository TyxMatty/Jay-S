//! `cargo run --release --example split -- <exam> [answer key]`: splits a real exam into questions
//! and applies its key, printing what came out. For checking the rules on real exams.
use jay_study::{exam, extract};

fn text(path: &str) -> String {
    let bytes = std::fs::read(path).expect("read");
    let format = extract::detect(&bytes[..bytes.len().min(4096)]).expect("format");
    extract::extract(format, bytes).expect("extract").text
}

fn main() {
    let mut args = std::env::args().skip(1);
    let exam_path = args.next().expect("exam");
    let mut questions = exam::split_questions(&text(&exam_path));
    let numbers: Vec<u32> = questions.iter().map(|q| q.number).collect();
    println!("{} questions: {numbers:?}", questions.len());
    let short: Vec<u32> = questions.iter().filter(|q| q.options.len() < 5).map(|q| q.number).collect();
    println!("with fewer than 5 options: {short:?}");
    println!("with a shared text: {}", questions.iter().filter(|q| q.passage.is_some()).count());
    if let Some(key_path) = args.next() {
        let key = exam::parse_answer_key(&text(&key_path)).expect("not a key");
        let applied = exam::apply_answer_key(&mut questions, &key);
        println!("key: {} entries, applied to {applied} questions", key.len());
    }
    let show: Vec<usize> = std::env::var("SHOW").ok().map(|s| s.split(',').filter_map(|n| n.parse().ok()).collect()).unwrap_or_default();
    for i in show {
        if let Some(q) = questions.get(i) {
            println!("--- #{i}: {}", serde_json::to_string_pretty(q).unwrap());
        }
    }
}
