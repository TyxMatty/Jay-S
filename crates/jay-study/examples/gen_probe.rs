//! `cargo run --release --example gen_probe -- <data folder> <content file> <style> [n]`: asks the
//! real model for `n` questions from the content, printing each with its time. For judging quality.
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::{exam, extract, generate};
use std::sync::atomic::AtomicBool;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let paths = AiPaths::new(std::path::Path::new(&args[1]));
    let bytes = std::fs::read(&args[2]).expect("content");
    let format = extract::detect(&bytes[..bytes.len().min(4096)]).expect("format");
    let text = extract::extract(format, bytes).expect("extract").text;
    let style = exam::style(&args[3]).expect("style");
    let n: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2);
    let chunks = generate::chunks(&text, 5000);
    println!("{} chunks", chunks.len());
    let t = std::time::Instant::now();
    let server = Server::start(&paths, ModelChoice::Standard).expect("server");
    println!("server up in {:?}", t.elapsed());
    let cancel = AtomicBool::new(false);
    let mut avoid = vec![];
    let (mut kept, mut answers) = (0, std::collections::BTreeMap::<String, usize>::new());
    for i in 0..n {
        let chunk = &chunks[i % chunks.len()];
        let focuses = generate::focuses(chunk);
        let focus = focuses.get((i / chunks.len()) % focuses.len().max(1)).map(String::as_str);
        let t = std::time::Instant::now();
        let result = generate::new_question(&server, &style, chunk, focus, &avoid, i as u32 + 1, "probe", 3, &cancel);
        println!("--- question {} in {:?}", i + 1, t.elapsed());
        match result {
            Ok(Some(q)) => {
                kept += 1;
                *answers.entry(q.answer.clone().unwrap()).or_default() += 1;
                avoid.push(q.stem.clone());
                println!("{}", serde_json::to_string_pretty(&q).unwrap());
            }
            Ok(None) => println!("no verified question after 3 tries"),
            Err(e) => println!("ERROR {e:?}"),
        }
    }
    println!("kept {kept} of {n}; answers {answers:?}");
    let _ = cancel;
}
