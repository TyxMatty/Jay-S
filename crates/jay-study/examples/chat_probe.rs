//! `cargo run --release --example chat_probe -- <data folder> <file>…`: the chat's acceptance check
//! with the real model. Opens a throwaway store under the temp dir, imports the files, asks a fixed
//! list of questions in one conversation scoped to them, then a few in a conversation with no
//! sources, and prints the kind, the time and every sentence with its status. Only Jay-AI's runtime
//! and model are read from the data folder; nothing is written there.
use jay_study::Store;
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::chat::Scope;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const SCOPED: [&str; 10] = [
    "Oi, tudo bem?",
    "Como eu faço um simulado no Jay-S?",
    "Para que serve o modo aprender?",
    "Pode fazer um resumo em tópicos desse conteúdo que anexei?",
    "O que é um intervalo de confiança?",
    "E quando o desvio padrão é desconhecido?",
    "Qual é a capital da França?",
    "Qual o tamanho da amostra para estimar uma proporção com 95% de confiança, erro de 2% e estimativa inicial de 10%?",
    "Com erro relativo de 10%, média 40 e desvio padrão 10, a 95%, qual o tamanho da amostra?",
    "Voltando ao começo: qual foi a definição de intervalo de confiança que você me deu?",
];

const GENERAL: [&str; 3] = ["Como anexo um PDF?", "Quanto é (1,96 * 10 / 4)^2?", "Como faço flashcards de uma prova que fiz?"];

/// A conversation a student really had, in a chat with no sources: a nickname for Jay-AI, then a
/// correction when it was misread.
const PERSONAL: [&str; 5] = ["Opa! Como vc ta?", "Pode se chamar de Matty?", "Eu to me referindo a você se chamar de Matty", "Qual é o seu nome mesmo?", "O que você consegue fazer?"];

/// Another conversation the student had, with no sources: an app question that wants a short
/// answer, the nickname, and a request for code that is not a calculation.
const MIXED: [&str; 4] = ["Como faço flashcards de uma prova que fiz?", "Posso te chamar de Matty?", "Como é seu apelido?", "Consegue escrever uma calculadora simples em py?"];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = std::env::temp_dir().join(format!("jay-s-chat-probe-{}", jay_study::id::new_id()));
    let store = Store::open(&root).expect("store");
    let materials: Vec<String> = args[2..].iter().map(|f| store.import(std::path::Path::new(f)).expect("import").id).collect();
    let server = Server::start(&AiPaths::new(std::path::Path::new(&args[1])), ModelChoice::Standard).expect("server");
    let cancel = AtomicBool::new(false);
    let chats = [(Scope { materials, attempt: None }, &SCOPED[..]), (Scope::default(), &GENERAL[..]), (Scope::default(), &PERSONAL[..]), (Scope::default(), &MIXED[..])];
    // Only one conversation, for repeated runs: PROBE_GENERAL=1 (the second), PROBE_PERSONAL=1 (the
    // third), PROBE_MIXED=1 (the fourth).
    let only = [("PROBE_GENERAL", 1), ("PROBE_PERSONAL", 2), ("PROBE_MIXED", 3)].iter().find(|(v, _)| std::env::var_os(v).is_some()).map(|&(_, i)| i);
    for (i, (scope, questions)) in chats.into_iter().enumerate() {
        if only.is_some_and(|o| o != i) {
            continue;
        }
        let chat = store.new_chat(scope).expect("chat");
        println!("=== chat: {}", if chat.title.is_empty() { "(no sources)" } else { &chat.title });
        for q in questions {
            let t = Instant::now();
            match store.chat_ask(&chat.id, q, &server, &cancel) {
                Ok(turn) => {
                    let r = &turn.reply;
                    println!("\n> {q}\n  kind={:?} go={:?} list={} time={:.1}s retried={} second_reading={} rejected_quotes={} sources={}", r.kind, r.go, r.list, t.elapsed().as_secs_f32(), r.checks.retried, r.checks.second_reading, r.checks.rejected_quotes, r.sources.iter().map(|s| s.label.as_str()).collect::<Vec<_>>().join(" | "));
                    for s in &r.sentences {
                        println!("  [{:?} {:?}] {}", s.status, s.sources, s.text);
                    }
                    if let Some(c) = &r.code {
                        println!("  CODE[{}]<<<\n{}\n>>>CODE", c.language, c.text);
                    }
                }
                Err(e) => println!("\n> {q}\n  ERROR {e:?} time={:.1}s", t.elapsed().as_secs_f32()),
            }
        }
        let c = store.chat(&chat.id).expect("reload");
        println!("\n--- summary (covers {} turns): {}\n--- facts: {:?}", c.summarized, c.summary, c.facts);
    }
    let _ = std::fs::remove_dir_all(root);
}
