//! `cargo run --release --example signals -- <file>… > signals.csv`: one CSV row per file with the
//! numbers `classify` decides on and what the rules answered, for training the content-vs-exam
//! model (docs/ARQUITETURA.md, stage 8). The label is the name of the file's folder (`exam/x.pdf`,
//! `content/y.pdf`, `both/z.pdf`). Only the file name goes out, never the path or the text.
use jay_study::classify::{classify, signals};
use jay_study::extract;

/// A CSV field: quoted when it has a comma, a quote or a line break.
fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_owned() }
}

fn main() {
    assert_eq!(field("a,\"b\".pdf"), "\"a,\"\"b\"\".pdf\"");
    println!("file,label,choice_questions,sum_questions,headers,exam_words,chars,density,first_at,rule_kind,rule_sure");
    for arg in std::env::args().skip(1) {
        let path = std::path::Path::new(&arg);
        let name = |p: Option<&std::ffi::OsStr>| p.map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{arg}: {e}");
                continue;
            }
        };
        let text = match extract::detect(&bytes[..bytes.len().min(4096)]).map(|f| extract::extract(f, bytes)) {
            Some(Ok(out)) => out.text,
            Some(Err(e)) => {
                eprintln!("{arg}: {e}");
                continue;
            }
            None => {
                eprintln!("{arg}: unsupported");
                continue;
            }
        };
        let (s, c) = (signals(&text), classify(&text));
        let kind = serde_json::to_string(&c.kind).unwrap();
        println!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            field(&name(path.file_name())),
            field(&name(path.parent().and_then(|p| p.file_name()))),
            s.choice_questions,
            s.sum_questions,
            s.headers,
            s.exam_words,
            s.chars,
            s.density,
            s.first_at,
            kind.trim_matches('"'),
            c.sure
        );
    }
}
