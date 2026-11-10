//! `cargo run --release --example probe -- <file>…`: reads each file like an import does and prints
//! the time, the classification and the start of the text. For checking the rules on real exams.
use jay_study::{classify, exam, extract};

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read");
        let t = std::time::Instant::now();
        let Some(format) = extract::detect(&bytes[..bytes.len().min(4096)]) else {
            println!("{path}: unsupported");
            continue;
        };
        match extract::extract(format, bytes) {
            Ok(out) => {
                let c = classify::classify(&out.text);
                println!("{path}: {:?} {} pages, {} ocr, {} chars, {:?} in {:?}", format, out.pages, out.ocr_pages, out.text.len(), c, t.elapsed());
                println!("warnings: {:?}", out.warnings);
                if let Some(key) = exam::parse_answer_key(&out.text) {
                    println!("key: answers for {} question numbers", key.len());
                }
                if std::env::var("SHOW").is_ok() {
                    println!("{}", out.text.chars().take(std::env::var("SHOW").unwrap().parse().unwrap_or(3000)).collect::<String>());
                }
            }
            Err(e) => println!("{path}: error {e}"),
        }
    }
}
