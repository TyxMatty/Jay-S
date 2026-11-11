//! `cargo run --release --example reason_bench -- <data folder> <lecture 15> <16> <17> <18>`: the
//! chat's reasoning benchmark with the real model. 25 questions with one known answer each, scored by
//! code: a number within a tolerance (every listed number for an interval), or keywords for the
//! non-numeric ones. Each question is asked in a new conversation (no history to lean on): the
//! "lecture" group in a chat scoped to the four Statistics lectures, the others with no sources.
//! Only our questions and expected answers are here; the lectures are read through the engine.
//! The expected numbers were worked out from the lectures' own formulas and checked with
//! `jay_study::calc` (see `expected_numbers_hold`, below `main`).
use jay_study::Store;
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::chat::Scope;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

enum Expect {
    /// Every one of these numbers must appear (within the tolerance).
    Numbers(&'static [f64], f64),
    /// Any of `yes` must appear and none of `no` (in the folded text, first two sentences).
    Words(&'static [&'static str], &'static [&'static str]),
}
use Expect::*;

/// (group, scoped to the lectures, question, expected)
const QUESTIONS: [(&str, bool, &str, Expect); 25] = [
    ("lecture", true, "Qual o tamanho da amostra para estimar uma proporção com 95% de confiança, erro de 2% e estimativa inicial de 10%?", Numbers(&[865.0], 0.0)),
    ("lecture", true, "Com erro relativo de 10%, média amostral 40 e desvio padrão 10, a 95% de confiança, qual o tamanho da amostra?", Numbers(&[25.0], 0.0)),
    ("lecture", true, "Controlando o erro absoluto w = 5 s, com desvio padrão de 12 s e 95% de confiança, qual o tamanho da amostra?", Numbers(&[23.0], 0.0)),
    ("lecture", true, "Numa população de 100 processos, com erro relativo de 12,5%, média 55 s, desvio padrão 13 s e 95% de confiança, quantos processos devem ser executados?", Numbers(&[13.0], 0.0)),
    ("lecture", true, "Em 500 usuários entrevistados, 260 usam o sistema. Qual o intervalo de confiança de 95% para a proporção?", Numbers(&[0.476, 0.564], 0.002)),
    ("lecture", true, "Uma amostra de 100 processos teve tempo médio de 40 ms, com desvio padrão populacional 10 ms. Qual o intervalo de confiança de 99% para a média?", Numbers(&[37.42, 42.58], 0.03)),
    ("lecture", true, "Uma amostra de 20 processos teve média 40 ms e desvio padrão amostral 10 ms. Qual o intervalo de confiança de 95% para a média?", Numbers(&[35.33, 44.67], 0.08)),
    ("lecture", true, "No país A, 312 de 430 consumidores usam a marca; no país B, 243 de 400. Qual o intervalo de confiança de 95% para a diferença entre as proporções?", Numbers(&[0.0543, 0.1819], 0.003)),
    ("lecture", true, "Duas amostras de monitores: A com 42 unidades, média 3300 dias e desvio populacional 283,2; B com 37, média 3050 e desvio populacional 284,6. Qual o intervalo de confiança de 99% para a diferença das médias?", Numbers(&[84.8, 415.2], 2.5)),
    ("lecture", true, "Numa amostra de 15 elementos, o desvio padrão da população é desconhecido. Para o intervalo de confiança da média, uso Z ou t?", Words(&["t de student", "distribuicao t", "estatistica t", "use t", "usa t", "se usa t", "usar t", "usa se t", "valor t", " o t ", "a t "], &[])),
    ("lecture", true, "Uma amostra de 5 medidas teve média 13,86 e desvio padrão amostral 0,658. Testando H0: média = 12 contra média diferente de 12, a 1% de significância, com t crítico 4,604, rejeito ou não H0?", Words(&["rejeit"], &["nao rejeit", "nao se rejeit", "aceit"])),
    ("lecture", true, "O fabricante diz que a média é pelo menos 500. Uma amostra de 45 deu média 490 e desvio padrão 14,2. A 5% de significância (Z crítico -1,64), rejeito H0: média = 500 a favor de média menor que 500?", Words(&["rejeit"], &["nao rejeit", "nao se rejeit", "aceit"])),
    ("school", false, "Quanto é 15% de 240?", Numbers(&[36.0], 0.0)),
    ("school", false, "Se 3 cadernos custam R$ 18, quanto custam 7 cadernos?", Numbers(&[42.0], 0.0)),
    ("school", false, "R$ 2.000 aplicados a juros simples de 3% ao mês por 5 meses: qual o montante?", Numbers(&[2300.0], 0.0)),
    ("school", false, "R$ 1.000 a juros compostos de 10% ao ano por 3 anos: qual o montante?", Numbers(&[1331.0], 0.0)),
    ("school", false, "Quantos metros por segundo são 72 km/h?", Numbers(&[20.0], 0.0)),
    ("school", false, "Qual a mediana de 3, 7, 1, 9, 5 e 4?", Numbers(&[4.5], 0.0)),
    ("school", false, "Qual a média de 4, 8, 15, 16, 23 e 42?", Numbers(&[18.0], 0.0)),
    ("school", false, "Lançando dois dados, qual a probabilidade de a soma ser 7? Dê em porcentagem.", Numbers(&[16.67], 0.05)),
    ("school", false, "Uma loja dá 20% de desconto num produto de R$ 150 e depois cobra 10% de imposto sobre o preço com desconto. Quanto o cliente paga?", Numbers(&[132.0], 0.0)),
    ("logic", false, "Todo estatístico é cuidadoso. Ana é estatística. Ana é cuidadosa? Responda sim ou não e por quê.", Words(&["sim"], &["nao e cuidadosa", "nao podemos", "nao se pode"])),
    ("logic", false, "Ana é mais alta que Bia, Bia é mais alta que Carla e Davi é mais baixo que Carla. Quem é o mais baixo?", Words(&["davi"], &[])),
    ("logic", false, "Se chove, a rua fica molhada. Hoje a rua está seca. Choveu hoje?", Words(&["nao choveu", "nao, ", "nao "], &["sim, choveu"])),
    ("logic", true, "Por que no intervalo de confiança da média se usa t em vez de Z?", Words(&["desconhecid"], &[])),
];

/// Lowercase, no accents, one space between words (the comma of "não, " kept as ", ").
fn fold(s: &str) -> String {
    let map = |c: char| match c {
        'á' | 'à' | 'â' | 'ã' => 'a',
        'é' | 'ê' => 'e',
        'í' => 'i',
        'ó' | 'ô' | 'õ' => 'o',
        'ú' => 'u',
        'ç' => 'c',
        // "usa-se t" reads as "usa se t".
        '-' => ' ',
        c => c,
    };
    format!(" {} ", s.to_lowercase().chars().map(map).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Every number in the text, read both ways when a dot could be a thousands separator ("1.331").
fn numbers(text: &str) -> Vec<f64> {
    let mut out = vec![];
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || ((chars[i] == '.' || chars[i] == ',') && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())) {
            i += 1;
        }
        let token: String = chars[start..i].iter().collect();
        let negative = start > 0 && (chars[start - 1] == '-' || chars[start - 1] == '−');
        let sign = if negative { -1.0 } else { 1.0 };
        let mut readings = vec![];
        if token.contains(',') {
            readings.push(token.replace('.', "").replace(',', "."));
        } else {
            readings.push(token.clone());
            readings.push(token.replace('.', ""));
        }
        out.extend(readings.iter().filter_map(|r| r.parse::<f64>().ok()).map(|v| v * sign));
    }
    out
}

fn right(expect: &Expect, text: &str) -> bool {
    match expect {
        Numbers(want, tol) => {
            let got = numbers(text);
            want.iter().all(|w| got.iter().any(|g| (g - w).abs() <= tol + 1e-9))
        }
        Words(yes, no) => {
            let t = fold(text);
            yes.iter().any(|w| t.contains(w)) && !no.iter().any(|w| t.contains(w))
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = std::env::temp_dir().join(format!("jay-s-reason-bench-{}", jay_study::id::new_id()));
    let store = Store::open(&root).expect("store");
    let materials: Vec<String> = args[2..].iter().map(|f| store.import(std::path::Path::new(f)).expect("import").id).collect();
    let server = Server::start(&AiPaths::new(std::path::Path::new(&args[1])), ModelChoice::Standard).expect("server");
    let cancel = AtomicBool::new(false);
    let mut totals: Vec<(&str, usize, usize, f32)> = vec![];
    for (i, (group, scoped, question, expect)) in QUESTIONS.iter().enumerate() {
        let scope = if *scoped { Scope { materials: materials.clone(), attempt: None } } else { Scope::default() };
        let chat = store.new_chat(scope).expect("chat");
        let t = Instant::now();
        let (ok, kind, text) = match store.chat_ask(&chat.id, question, &server, &cancel) {
            Ok(turn) => {
                let text: Vec<String> = turn.reply.sentences.iter().map(|s| s.text.clone()).collect();
                // Keywords are judged on the opening (the answer), numbers on the whole reply.
                let judged = if matches!(expect, Words(..)) { text.iter().take(2).cloned().collect::<Vec<_>>().join(" ") } else { text.join(" ") };
                (right(expect, &judged), format!("{:?}", turn.reply.kind), text.join(" | "))
            }
            Err(e) => (false, "Error".into(), format!("{e:?}")),
        };
        let secs = t.elapsed().as_secs_f32();
        println!("{:>2} {:7} {} {:5.1}s {:8} {}\n   {}", i + 1, group, if ok { "RIGHT" } else { "wrong" }, secs, kind, question, text.chars().take(600).collect::<String>());
        match totals.iter_mut().find(|t| t.0 == *group) {
            Some(t) => {
                t.1 += ok as usize;
                t.2 += 1;
                t.3 += secs;
            }
            None => totals.push((group, ok as usize, 1, secs)),
        }
    }
    let (r, n, s) = totals.iter().fold((0, 0, 0.0), |a, t| (a.0 + t.1, a.1 + t.2, a.2 + t.3));
    for (g, r, n, s) in &totals {
        println!("== {g}: {r}/{n} right, {:.1} s per question", s / *n as f32);
    }
    println!("== total: {r}/{n} right, {:.1} s per question", s / n as f32);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_numbers_hold() {
        let e = |x: &str| jay_study::calc::eval(x).unwrap();
        assert_eq!(e("teto(1,96^2*0,1*0,9/0,02^2)"), 865.0);
        assert_eq!(e("teto((1,96*10/(0,1*40))^2)"), 25.0);
        assert_eq!(e("teto((1,96*12/5)^2)"), 23.0);
        let n0 = e("(1,96*13/(0,125*55))^2");
        assert_eq!((n0 / (1.0 + n0 / 100.0)).ceil(), 13.0);
        let m = e("1,96*sqrt(0,52*0,48/500)");
        assert!((0.52 - m - 0.4762).abs() < 0.001 && (0.52 + m - 0.5638).abs() < 0.001);
        assert!((e("2,58*10/sqrt(100)") - 2.58).abs() < 1e-9);
        assert!((e("2,093*10/sqrt(20)") - 4.680).abs() < 0.001);
        let d = e("1,96*sqrt(312/430*(1-312/430)/430+243/400*(1-243/400)/400)");
        assert!((0.7256 - 0.6075 - d - 0.0543).abs() < 0.002 && (0.7256 - 0.6075 + d - 0.1819).abs() < 0.002);
        // The lecture prints ± 167,1 (82,9 to 417,1); the formula gives 165,2: both are within the tolerance.
        assert!((e("2,58*sqrt(283,2^2/42+284,6^2/37)") - 165.2).abs() < 0.1);
        assert_eq!(e("240*15/100"), 36.0);
        assert_eq!(e("18/3*7"), 42.0);
        assert_eq!(e("2000*(1+0,03*5)"), 2300.0);
        assert!((e("1000*1,1^3") - 1331.0).abs() < 1e-9);
        assert_eq!(e("72/3,6"), 20.0);
        assert_eq!(e("(4+8+15+16+23+42)/6"), 18.0);
        assert!((e("6/36*100") - 16.67).abs() < 0.01);
        assert!((e("150*0,8*1,1") - 132.0).abs() < 1e-9);
        assert!((e("(13,86-12)/(0,658/sqrt(5))") - 6.32).abs() < 0.01);
        assert!((e("(490-500)/(14,2/sqrt(45))") + 4.724).abs() < 0.001);
    }

    #[test]
    fn numbers_and_words_are_scored() {
        assert!(right(&Numbers(&[1331.0], 0.0), "O montante é R$ 1.331,00."));
        assert!(right(&Numbers(&[1331.0], 0.0), "O montante é 1.331."));
        assert!(right(&Numbers(&[0.476, 0.564], 0.002), "IC = 0,4762 ≤ p ≤ 0,5638"));
        assert!(right(&Numbers(&[16.67], 0.05), "A probabilidade é 1/6, ou 16,67%."));
        assert!(!right(&Numbers(&[25.0], 0.0), "n = 38416"));
        assert!(right(&Words(&["rejeit"], &["nao rejeit"]), "Rejeita-se H0, pois t > 4,604."));
        assert!(!right(&Words(&["rejeit"], &["nao rejeit"]), "Não rejeitamos H0."));
        assert!(right(&QUESTIONS[9].3, "Quando o desvio padrão é desconhecido, usa-se t."));
    }
}
