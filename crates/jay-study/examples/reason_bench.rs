//! `cargo run --release --example reason_bench -- <data folder> <lecture 15> <16> <17> <18>`: the
//! chat's reasoning benchmark with the real model. 42 questions with one known answer each, scored by
//! code: a number within a tolerance (every listed number for an interval), or keywords for the
//! non-numeric ones (judged on the reply's first sentence, the answer: a hedge does not count). Each
//! question is asked in a new conversation (no history to lean on), once per seed (`BENCH_SEEDS`,
//! default `1,2,3`; the server's fixed seed makes a run repeat), the lecture ones in a chat scoped to
//! the four Statistics lectures, the others with no sources. Groups: `copied` (the lectures' own worked
//! examples), `novel` (the same kinds with other numbers, so the answer is in no lecture), `school`,
//! `logic`; `BENCH_ONLY=novel,logic` runs some. Only our questions and expected answers are here; the
//! lectures are read through the engine. The expected numbers were worked out from the lectures' own
//! formulas and checked with `jay_study::calc` (see `expected_numbers_hold`, below `main`).
use jay_study::Store;
use jay_study::ai::{AiPaths, ModelChoice, Server};
use jay_study::chat::Scope;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

enum Expect {
    /// Every one of these numbers must appear (within the tolerance).
    Numbers(&'static [f64], f64),
    /// Any of `yes` must appear and none of `no` (in the folded first sentence; a `no` word right
    /// after "não" doesn't count, so "não rejeitamos" is not "rejeitamos").
    Words(&'static [&'static str], &'static [&'static str]),
}
use Expect::*;

/// The words of a wrong answer to "reject H0?" when the right one is to reject.
const NOT_REJECT: &[&str] = &["nao rejeit", "nao se rejeit", "nao ha evidencia", "nao e possivel", "nao podemos", "nao devemos", "nao se deve", "aceit"];

/// (group, scoped to the lectures, question, expected)
const QUESTIONS: [(&str, bool, &str, Expect); 42] = [
    ("copied", true, "Qual o tamanho da amostra para estimar uma proporção com 95% de confiança, erro de 2% e estimativa inicial de 10%?", Numbers(&[865.0], 0.0)),
    ("copied", true, "Com erro relativo de 10%, média amostral 40 e desvio padrão 10, a 95% de confiança, qual o tamanho da amostra?", Numbers(&[25.0], 0.0)),
    ("copied", true, "Controlando o erro absoluto w = 5 s, com desvio padrão de 12 s e 95% de confiança, qual o tamanho da amostra?", Numbers(&[23.0], 0.0)),
    ("copied", true, "Numa população de 100 processos, com erro relativo de 12,5%, média 55 s, desvio padrão 13 s e 95% de confiança, quantos processos devem ser executados?", Numbers(&[13.0], 0.0)),
    ("copied", true, "Em 500 usuários entrevistados, 260 usam o sistema. Qual o intervalo de confiança de 95% para a proporção?", Numbers(&[0.476, 0.564], 0.002)),
    ("copied", true, "Uma amostra de 100 processos teve tempo médio de 40 ms, com desvio padrão populacional 10 ms. Qual o intervalo de confiança de 99% para a média?", Numbers(&[37.42, 42.58], 0.03)),
    ("copied", true, "Uma amostra de 20 processos teve média 40 ms e desvio padrão amostral 10 ms. Qual o intervalo de confiança de 95% para a média?", Numbers(&[35.33, 44.67], 0.08)),
    ("copied", true, "No país A, 312 de 430 consumidores usam a marca; no país B, 243 de 400. Qual o intervalo de confiança de 95% para a diferença entre as proporções?", Numbers(&[0.0543, 0.1819], 0.003)),
    ("copied", true, "Duas amostras de monitores: A com 42 unidades, média 3300 dias e desvio populacional 283,2; B com 37, média 3050 e desvio populacional 284,6. Qual o intervalo de confiança de 99% para a diferença das médias?", Numbers(&[84.8, 415.2], 2.5)),
    ("copied", true, "Numa amostra de 15 elementos, o desvio padrão da população é desconhecido. Para o intervalo de confiança da média, uso Z ou t?", Words(&["t de student", "distribuicao t", "estatistica t", "use t", "usa t", "se usa t", "usar t", "usa se t", "valor t", " o t ", "a t "], &["usa se z", "use z", "usa z", "uso z", "usar z"])),
    ("copied", true, "Uma amostra de 5 medidas teve média 13,86 e desvio padrão amostral 0,658. Testando H0: média = 12 contra média diferente de 12, a 1% de significância, com t crítico 4,604, rejeito ou não H0?", Words(&["rejeit"], NOT_REJECT)),
    ("copied", true, "O fabricante diz que a média é pelo menos 500. Uma amostra de 45 deu média 490 e desvio padrão 14,2. A 5% de significância (Z crítico -1,64), rejeito H0: média = 500 a favor de média menor que 500?", Words(&["rejeit"], NOT_REJECT)),
    ("novel", true, "Qual o tamanho da amostra para estimar uma proporção com 95% de confiança, erro de 3% e estimativa inicial de 20%?", Numbers(&[683.0], 0.0)),
    ("novel", true, "Com erro relativo de 5%, média amostral 50 e desvio padrão 8, a 95% de confiança, qual o tamanho da amostra?", Numbers(&[40.0], 0.0)),
    ("novel", true, "Controlando o erro absoluto w = 3 s, com desvio padrão de 15 s e 95% de confiança, qual o tamanho da amostra?", Numbers(&[97.0], 0.0)),
    ("novel", true, "Numa população de 200 processos, com erro relativo de 10%, média 30 s, desvio padrão 9 s e 95% de confiança, quantos processos devem ser executados?", Numbers(&[30.0], 0.0)),
    ("novel", true, "Em 400 usuários entrevistados, 120 usam o sistema. Qual o intervalo de confiança de 95% para a proporção?", Numbers(&[0.2551, 0.3449], 0.003)),
    ("novel", true, "Uma amostra de 64 processos teve tempo médio de 50 ms, com desvio padrão populacional 12 ms. Qual o intervalo de confiança de 99% para a média?", Numbers(&[46.13, 53.87], 0.05)),
    ("novel", true, "Uma amostra de 16 processos teve média 25 ms e desvio padrão amostral 4 ms. Qual o intervalo de confiança de 95% para a média?", Numbers(&[22.869, 27.131], 0.03)),
    ("novel", true, "No país A, 150 de 300 consumidores usam a marca; no país B, 120 de 300. Qual o intervalo de confiança de 95% para a diferença entre as proporções?", Numbers(&[0.0208, 0.1792], 0.003)),
    ("novel", true, "Duas amostras de monitores: A com 50 unidades, média 120 dias e desvio populacional 10; B com 40, média 115 e desvio populacional 12. Qual o intervalo de confiança de 99% para a diferença das médias?", Numbers(&[-1.105, 11.105], 0.05)),
    ("novel", true, "Uma amostra de 9 medidas teve média 52 e desvio padrão amostral 3. Testando H0: média = 50 contra média diferente de 50, a 5% de significância, com t crítico 2,306, rejeito ou não H0?", Words(&["nao rejeit", "nao se rejeit", "nao ha evidencia"], &["rejeita se h0", "rejeitamos h0", "rejeito h0"])),
    ("novel", true, "O fabricante diz que a média é pelo menos 100. Uma amostra de 36 deu média 98 e desvio padrão 6. A 5% de significância (Z crítico -1,64), rejeito H0: média = 100 a favor de média menor que 100?", Words(&["rejeit"], NOT_REJECT)),
    ("school", false, "Quanto é 15% de 240?", Numbers(&[36.0], 0.0)),
    ("school", false, "Se 3 cadernos custam R$ 18, quanto custam 7 cadernos?", Numbers(&[42.0], 0.0)),
    ("school", false, "R$ 2.000 aplicados a juros simples de 3% ao mês por 5 meses: qual o montante?", Numbers(&[2300.0], 0.0)),
    ("school", false, "R$ 1.000 a juros compostos de 10% ao ano por 3 anos: qual o montante?", Numbers(&[1331.0], 0.0)),
    ("school", false, "Quantos metros por segundo são 72 km/h?", Numbers(&[20.0], 0.0)),
    ("school", false, "Qual a mediana de 3, 7, 1, 9, 5 e 4?", Numbers(&[4.5], 0.0)),
    ("school", false, "Qual a média de 4, 8, 15, 16, 23 e 42?", Numbers(&[18.0], 0.0)),
    ("school", false, "Lançando dois dados, qual a probabilidade de a soma ser 7? Dê em porcentagem.", Numbers(&[16.67], 0.05)),
    ("school", false, "Uma loja dá 20% de desconto num produto de R$ 150 e depois cobra 10% de imposto sobre o preço com desconto. Quanto o cliente paga?", Numbers(&[132.0], 0.0)),
    ("logic", false, "Todo estatístico é cuidadoso. Ana é estatística. Ana é cuidadosa? Responda sim ou não e por quê.", Words(&[" sim"], &["nao e cuidadosa", "nao podemos", "nao se pode", "nao e possivel", "nao necessariamente", "nao posso"])),
    ("logic", false, "Ana é mais alta que Bia, Bia é mais alta que Carla e Davi é mais baixo que Carla. Quem é o mais baixo?", Words(&["davi"], &["carla e a mais baixa", "bia e a mais baixa", "ana e a mais baixa"])),
    ("logic", false, "Se chove, a rua fica molhada. Hoje a rua está seca. Choveu hoje?", Words(&["nao choveu", " nao. ", " nao! ", " nao, nao "], &["nao podemos", "nao e possivel", "nao da para", "talvez", "nao se pode", " sim"])),
    ("logic", true, "Por que no intervalo de confiança da média se usa t em vez de Z?", Words(&["desconhecid"], &["nao e desconhecid", "nao desconhecid"])),
    ("logic", false, "Todo peixe voa. Nemo é um peixe. Pela lógica dessas premissas, Nemo voa?", Words(&[" sim"], &["nao podemos", "nao e possivel", "nao se pode"])),
    ("logic", false, "Se um número é múltiplo de 4, então ele é par. O 7 não é par. O 7 é múltiplo de 4?", Words(&["nao e multiplo", "nao, ", " nao. "], &["nao podemos", "nao e possivel", "talvez"])),
    ("logic", false, "Alguns alunos são atletas. Todos os atletas acordam cedo. Dá para concluir que todos os alunos acordam cedo?", Words(&["nao"], &["sim, todos", "sim. "])),
    ("logic", false, "O ônibus sai antes do trem. O trem sai antes do avião. O carro sai depois do avião, e a bicicleta sai antes do ônibus. Qual sai primeiro?", Words(&["bicicleta"], &[])),
    ("logic", false, "A chave está na gaveta ou na mochila. Não está na gaveta. Onde está a chave?", Words(&["mochila"], &["gaveta ou"])),
    ("logic", false, "Numa sala há 5 pessoas, e cada uma aperta a mão de cada uma das outras uma vez. Quantos apertos de mão acontecem?", Numbers(&[10.0], 0.0)),
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

/// `w` in the folded text `t`, unless it only appears right after "não" (a `w` that itself starts with "nao" is plain).
fn says(t: &str, w: &str) -> bool {
    t.match_indices(w).any(|(i, _)| w.starts_with("nao") || !(t[..i].ends_with("nao ") || t[..i].ends_with("nao se ")))
}

fn right(expect: &Expect, text: &str) -> bool {
    match expect {
        Numbers(want, tol) => {
            let got = numbers(text);
            want.iter().all(|w| got.iter().any(|g| (g - w).abs() <= tol + 1e-9))
        }
        Words(yes, no) => {
            let t = fold(text);
            yes.iter().any(|w| t.contains(w)) && !no.iter().any(|w| says(&t, w))
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seeds: Vec<u32> = std::env::var("BENCH_SEEDS").unwrap_or_else(|_| "1,2,3".into()).split(',').map(|s| s.trim().parse().expect("BENCH_SEEDS: numbers separated by commas")).collect();
    let only = std::env::var("BENCH_ONLY").ok();
    let chosen = |group: &str| only.as_ref().is_none_or(|o| o.split(',').any(|g| g.trim() == group));
    let root = std::env::temp_dir().join(format!("jay-s-reason-bench-{}", jay_study::id::new_id()));
    let store = Store::open(&root).expect("store");
    let materials: Vec<String> = args[2..].iter().map(|f| store.import(std::path::Path::new(f)).expect("import").id).collect();
    let server = Server::start(&AiPaths::new(std::path::Path::new(&args[1])), ModelChoice::Standard).expect("server");
    let cancel = AtomicBool::new(false);
    // results[question] = one verdict per seed
    let mut results: Vec<Vec<bool>> = vec![vec![]; QUESTIONS.len()];
    let mut seconds = 0.0;
    for &seed in &seeds {
        server.set_seed(Some(seed));
        for (i, (group, scoped, question, expect)) in QUESTIONS.iter().enumerate().filter(|(_, q)| chosen(q.0)) {
            let scope = if *scoped { Scope { materials: materials.clone(), attempt: None } } else { Scope::default() };
            let chat = store.new_chat(scope).expect("chat");
            let t = Instant::now();
            let (ok, kind, text) = match store.chat_ask(&chat.id, question, &server, &cancel) {
                Ok(turn) => {
                    let text: Vec<String> = turn.reply.sentences.iter().map(|s| s.text.clone()).collect();
                    // Keywords are judged on the first sentence (the answer), numbers on the whole reply.
                    let judged = if matches!(expect, Words(..)) { text.first().cloned().unwrap_or_default() } else { text.join(" ") };
                    (right(expect, &judged), format!("{:?}", turn.reply.kind), text.join(" | "))
                }
                Err(e) => (false, "Error".into(), format!("{e:?}")),
            };
            let secs = t.elapsed().as_secs_f32();
            seconds += secs;
            results[i].push(ok);
            println!("seed {seed} {:>2} {:7} {} {:5.1}s {:8} {}\n   {}", i + 1, group, if ok { "RIGHT" } else { "wrong" }, secs, kind, question, text.chars().take(400).collect::<String>());
        }
    }
    println!();
    let mut total = vec![0usize; seeds.len()];
    for group in ["copied", "novel", "school", "logic"].into_iter().filter(|g| chosen(g)) {
        let mut sums = vec![0usize; seeds.len()];
        let mut n = 0;
        for (i, _) in QUESTIONS.iter().enumerate().filter(|(_, q)| q.0 == group) {
            let line: String = results[i].iter().map(|&ok| if ok { '#' } else { '.' }).collect();
            println!("{:02} {:7} {line}", i + 1, group);
            n += 1;
            for (s, &ok) in results[i].iter().enumerate() {
                sums[s] += ok as usize;
                total[s] += ok as usize;
            }
        }
        let list: Vec<String> = sums.iter().map(|s| s.to_string()).collect();
        println!("== {group}: {} → mean {:.2} / {n}", list.join(", "), sums.iter().sum::<usize>() as f32 / seeds.len() as f32);
    }
    let asked = results.iter().map(|r| r.len()).sum::<usize>();
    println!("== total: {} → mean {:.2} per seed, {:.1} s per question", total.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "), total.iter().sum::<usize>() as f32 / seeds.len() as f32, seconds / asked.max(1) as f32);
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
        // The novel twins.
        assert_eq!(e("teto(1,96^2*0,2*0,8/0,03^2)"), 683.0);
        assert_eq!(e("teto((1,96*8/(0,05*50))^2)"), 40.0);
        assert_eq!(e("teto((1,96*15/3)^2)"), 97.0);
        let n0 = e("(1,96*9/(0,1*30))^2");
        assert_eq!((n0 / (1.0 + n0 / 200.0)).ceil(), 30.0);
        let m = e("1,96*sqrt(0,3*0,7/400)");
        assert!((0.3 - m - 0.2551).abs() < 0.001 && (0.3 + m - 0.3449).abs() < 0.001);
        assert!((e("2,58*12/sqrt(64)") - 3.87).abs() < 0.001);
        assert!((e("2,131*4/sqrt(16)") - 2.131).abs() < 0.001);
        let d = e("1,96*sqrt(0,5*0,5/300+0,4*0,6/300)");
        assert!((0.1 - d - 0.0208).abs() < 0.001 && (0.1 + d - 0.1792).abs() < 0.001);
        let d = e("2,58*sqrt(10^2/50+12^2/40)");
        assert!((5.0 - d + 1.105).abs() < 0.001 && (5.0 + d - 11.105).abs() < 0.001);
        assert!((e("(52-50)/(3/sqrt(9))") - 2.0).abs() < 1e-9 && 2.0 < 2.306);
        assert!((e("(98-100)/(6/sqrt(36))") + 2.0).abs() < 1e-9 && -2.0 < -1.64);
        assert_eq!(e("5*4/2"), 10.0);
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
        assert!(!right(&QUESTIONS[9].3, "Usa-se Z, pois a amostra é pequena."));
        let rain = &QUESTIONS[34].3;
        assert!(!right(rain, "Não podemos afirmar com certeza se choveu."));
        assert!(right(rain, "Não choveu, porque a rua está seca."));
        assert!(!right(rain, "Sim, choveu."));
        let novel_test = &QUESTIONS[21].3;
        assert!(right(novel_test, "Não rejeitamos H0, pois t = 2,0 < 2,306."));
        assert!(right(novel_test, "Não rejeito H0."));
        assert!(!right(novel_test, "Rejeitamos H0."));
        let reject = &QUESTIONS[10].3;
        assert!(right(reject, "Rejeitamos H0, pois t = 6,32 > 4,604."));
        assert!(!right(reject, "Não há evidência para rejeitar H0."));
        assert!(!right(&QUESTIONS[32].3, "Não podemos afirmar que Ana seja cuidadosa."));
        assert!(right(&QUESTIONS[32].3, "Sim, Ana é cuidadosa. Assim concluímos."));
        assert!(right(&QUESTIONS[20].3, "O intervalo é de −1,105 a 11,105."));
        assert!(!right(&QUESTIONS[38].3, "Sim, todos os alunos acordam cedo."));
        assert!(right(&QUESTIONS[38].3, "Não, só dá para concluir isso dos atletas."));
        assert!(!right(&QUESTIONS[40].3, "A chave está na gaveta ou na mochila."));
    }
}
