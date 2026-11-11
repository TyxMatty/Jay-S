//! `cargo run --release --example uepg_key -- <key.pdf> [exam dir]`: reads UEPG's Vestibular 2025
//! answer-key edital and prints, as Rust source for `src/catalog.rs`, the sums of each catalogue
//! entry. A question is accepted only when its proposition row and its sum agree; the rest goes in
//! the "for a human" list. With the exams' folder, it also checks that each exam's subject order
//! matches the key's blocks. Dev-time only: the app never runs this.
use jay_study::classify::fold;
use jay_study::{exam, extract};

fn text(path: &str) -> String {
    let bytes = std::fs::read(path).expect("read");
    let format = extract::detect(&bytes[..bytes.len().min(4096)]).expect("format");
    extract::extract(format, bytes).expect("extract").text
}

/// (the last word of the key's subject title, folded) -> (the exam's heading, folded).
const SUBJECTS: [(&str, &str); 7] =
    [("portuguesa", "portugues"), ("matematica", "matematica"), ("fisica", "fisica"), ("quimica", "quimica"), ("biologia", "biologia"), ("historia", "historia"), ("geografia", "geografia")];

#[derive(Clone, PartialEq)]
enum Outcome {
    Sum(u32),
    Annulled,
}

struct Item {
    number: u32,
    outcome: Outcome,
    void: Vec<u32>,
}

/// One table of the key: its subject and groups (Anexo 2), or "cg" with its language (Anexo 1).
struct Block {
    label: String,
    subject: String,
    groups: Vec<u32>,
    language: Option<&'static str>,
    items: Vec<Item>,
}

/// (id, constant prefix, which blocks it takes, expected question numbers).
type CatalogEntry = (String, String, Box<dyn Fn(&Block) -> bool>, Vec<u32>);

/// A row's proposition marks ("- 02 04 - 16", "A 02 04 08 16") against its sum. `each` = propositions.
fn check(marks: &[&str], sum: &str, each: usize) -> Result<(Outcome, Vec<u32>), String> {
    let (mut total, mut void, mut all_dash) = (0u32, vec![], true);
    for (p, m) in marks.iter().enumerate() {
        let value = 1u32 << p;
        match *m {
            "-" => {}
            "A" => {
                void.push(value);
                all_dash = false;
            }
            m if m.len() == 2 && m.parse::<u32>().ok() == Some(value) => {
                total += value;
                all_dash = false;
            }
            m => return Err(format!("mark {m:?} at position {p}")),
        }
    }
    if marks.len() != each {
        return Err(format!("{} marks, expected {each}", marks.len()));
    }
    if sum == "ANULADA" {
        return if all_dash { Ok((Outcome::Annulled, void)) } else { Err("ANULADA but some proposition is marked".into()) };
    }
    match sum.parse::<u32>() {
        Ok(s) if sum.len() == 2 && s == total && total > 0 => Ok((Outcome::Sum(s), void)),
        _ => Err(format!("sum {sum} but the marked propositions add up to {total}")),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let key_text = text(&args.next().expect("key.pdf"));
    let exams_dir = args.next();
    let flat = key_text.replace('\x0c', "\n");
    let lines: Vec<&str> = flat.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let tokens: Vec<Vec<&str>> = lines.iter().map(|l| l.split_whitespace().collect()).collect();
    let is_number = |t: &str| t.trim_end_matches('*').len() == 2 && t.trim_end_matches('*').bytes().all(|b| b.is_ascii_digit());

    let (mut anexo, mut language, mut subject, mut groups) = (0, None::<&'static str>, String::new(), vec![]);
    let mut blocks: Vec<Block> = vec![];
    let mut human: Vec<String> = vec![];
    let mut i = 0;
    while i < lines.len() {
        let (line, toks) = (lines[i], &tokens[i]);
        if line.contains("PARECER FUNDAMENTADO") {
            break;
        }
        if line.contains("ANEXO 1 DO EDITAL") {
            (anexo, language, subject, groups) = (1, None, String::new(), vec![]);
        } else if line.contains("ANEXO 2 DO EDITAL") {
            (anexo, language, subject, groups) = (2, None, String::new(), vec![]);
        } else if anexo == 1 && line == "Inglês" {
            language = Some("ingles");
        } else if anexo == 1 && line == "Espanhol" {
            language = Some("espanhol");
        } else if line.starts_with("Grupo") {
            groups = line.split(|c: char| !c.is_ascii_digit()).filter_map(|s| s.parse().ok()).collect();
        } else if let Some(last) = toks.last().filter(|t| t.chars().all(char::is_uppercase) && SUBJECTS.iter().any(|(k, _)| *k == fold(t))) {
            subject = fold(last);
            groups.clear();
        }
        // A table is three lines: the numbers, the propositions, the sums.
        let each = if anexo == 1 { 4 } else { 5 };
        if anexo > 0 && !toks.is_empty() && toks.iter().all(|t| is_number(t)) && i + 2 < lines.len() {
            let (marks, sums) = (&tokens[i + 1], &tokens[i + 2]);
            let sums_ok = sums.len() == toks.len() && sums.iter().all(|s| is_number(s) || *s == "ANULADA");
            let marks_ok = marks.iter().all(|m| matches!(*m, "-" | "A") || is_number(m));
            if sums_ok && marks_ok && marks.len() == toks.len() * each {
                let label = if anexo == 1 { format!("cg{}", language.map(|l| format!("-{l}")).unwrap_or_default()) } else { format!("{subject} grupos {groups:?}") };
                if blocks.last().is_none_or(|b| b.label != label) {
                    blocks.push(Block { label: label.clone(), subject: subject.clone(), groups: groups.clone(), language, items: vec![] });
                }
                let block = blocks.last_mut().unwrap();
                for ((n, m), s) in toks.iter().zip(marks.chunks(each)).zip(sums.iter()) {
                    let number: u32 = n.trim_end_matches('*').parse().unwrap();
                    match check(m, s, each) {
                        Ok((outcome, void)) => block.items.push(Item { number, outcome, void }),
                        Err(why) => human.push(format!("{label} question {number}: {why}")),
                    }
                }
                i += 3;
                continue;
            }
        }
        i += 1;
    }

    // The parecer (the edital's own explanation of the changes) must say what the table says.
    let parecer: String = flat[flat.find("PARECER FUNDAMENTADO").unwrap_or(0)..].split_whitespace().collect::<Vec<_>>().join(" ");
    let confirm = |what: &str, phrase: &str| {
        let ok = parecer.contains(phrase);
        println!("parecer {} {what}: {phrase:?}", if ok { "confirms" } else { "DOES NOT CONFIRM" });
        if !ok {
            std::process::exit(1);
        }
    };
    let find = |subject: &str, n: u32| blocks.iter().filter(|b| b.subject == subject).flat_map(|b| &b.items).find(|i| i.number == n);
    let expect = |subject: &str, n: u32, sum: Outcome, void: &[u32]| {
        let item = find(subject, n).unwrap_or_else(|| panic!("{subject} {n} missing"));
        assert!(item.outcome == sum && item.void == void, "{subject} {n} is not what the parecer says");
    };
    expect("fisica", 33, Outcome::Annulled, &[]);
    confirm("Física 33 annulled", "a CPS decidiu ANULAR a questão 33");
    expect("fisica", 38, Outcome::Sum(30), &[1]);
    confirm("Física 38 prop 01 annulled, sum 30", "a CPS acatou o pedido de anulação da alternativa. Portanto, a somatória passa de 30");
    expect("fisica", 40, Outcome::Sum(12), &[1]);
    confirm("Física 40 prop 01 annulled, 16 false, sum 12", "a somatória da questão 40 passa a ser 12");
    expect("matematica", 24, Outcome::Sum(4), &[]);
    confirm("Matemática 24 sum 04", "a somatória passa de 06 para 04");
    expect("quimica", 11, Outcome::Sum(28), &[]);
    confirm("Química 11 sum 28", "a somatória passa de 20 para 28");

    // The 8 catalogue entries: (id, constant, groups, expected numbers).
    let entries: Vec<CatalogEntry> = {
        let mut v: Vec<CatalogEntry> = vec![];
        for g in 1..=6u32 {
            v.push((format!("uepg-2025-grupo-{g}"), format!("G{g}"), Box::new(move |b: &Block| b.groups.contains(&g)), (1..=45).collect()));
        }
        for (id, c, lang) in [("ingles", "CG_EN", "ingles"), ("espanhol", "CG_ES", "espanhol")] {
            v.push((format!("uepg-2025-cg-{id}"), c.into(), Box::new(move |b: &Block| b.label == "cg" || b.language == Some(lang)), (1..=60).collect()));
        }
        v
    };
    let (mut accepted, mut missing) = (0, 0);
    for (id, constant, belongs, expected) in &entries {
        let mut items: Vec<&Item> = blocks.iter().filter(|b| belongs(b)).flat_map(|b| &b.items).collect();
        items.sort_by_key(|i| i.number);
        let numbers: Vec<u32> = items.iter().map(|i| i.number).collect();
        let lost: Vec<u32> = expected.iter().filter(|n| !numbers.contains(n)).copied().collect();
        let twice: Vec<u32> = numbers.windows(2).filter(|w| w[0] == w[1]).map(|w| w[0]).collect();
        accepted += numbers.len();
        missing += lost.len();
        println!("\n// {id}: {} accepted, not accepted {lost:?}, twice {twice:?}", numbers.len());
        print!("const {constant}_ANSWERS: &[(u32, &str)] = &[");
        for (k, it) in items.iter().filter_map(|i| if let Outcome::Sum(s) = i.outcome { Some((i.number, s)) } else { None }).enumerate() {
            print!("{}({}, \"{:02}\"),", if k % 8 == 0 { "\n    " } else { " " }, it.0, it.1);
        }
        println!("\n];");
        let annulled: Vec<u32> = items.iter().filter(|i| i.outcome == Outcome::Annulled).map(|i| i.number).collect();
        let void: Vec<String> = items.iter().flat_map(|i| i.void.iter().map(move |v| format!("({}, \"{v:02}\")", i.number))).collect();
        println!("const {constant}_ANNULLED: &[u32] = &{annulled:?};\nconst {constant}_VOID: &[(u32, &str)] = &[{}];", void.join(", "));
    }
    println!("\nsummary: {accepted} accepted over the 8 entries (a shared question counts once per entry), {missing} missing, {} for a human", human.len());
    println!("blocks: {}", blocks.iter().map(|b| format!("{} ({})", b.label, b.items.len())).collect::<Vec<_>>().join("; "));
    println!("for a human:");
    for h in &human {
        println!("  {h}");
    }

    // The mapping of subjects to groups against each exam's own order of subjects.
    let Some(dir) = exams_dir else { return };
    println!("subject order, by the exams' own headings:");
    let files = ["grupo-1-por-mat-his", "grupo-2-por-mat-fis", "grupo-3-por-geo-his", "grupo-4-qui-bio-fis", "grupo-5-qui-mat-fis", "grupo-6-por-biol-his"];
    for (k, file) in files.iter().enumerate() {
        let g = k as u32 + 1;
        let exam_text = text(&format!("{dir}/uepg-2025-{file}.pdf"));
        let exam_lines: Vec<&str> = exam_text.split('\n').collect();
        // What the key says this group has, by question range.
        let mut mine: Vec<&Block> = blocks.iter().filter(|b| b.groups.contains(&g)).collect();
        mine.sort_by_key(|b| b.items.iter().map(|i| i.number).min());
        let heads: Vec<(usize, String)> = exam_lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.trim().chars().all(|c| !c.is_lowercase()))
            .filter_map(|(n, l)| SUBJECTS.iter().find(|(_, h)| fold(l.trim()) == *h).map(|(_, h)| (n, h.to_string())))
            .collect();
        let want: Vec<&str> = mine.iter().map(|b| SUBJECTS.iter().find(|(k, _)| *k == b.subject).unwrap().1).collect();
        let got: Vec<&str> = heads.iter().map(|(_, h)| h.as_str()).collect();
        assert_eq!(got, want, "grupo {g}: the exam's subject headings are not the key's blocks");
        for (b, block) in mine.iter().enumerate() {
            let end = heads.get(b + 1).map_or(exam_lines.len(), |h| h.0);
            let qs = exam::split_questions(&exam_lines[heads[b].0..end].join("\n"));
            let numbers: Vec<u32> = qs.iter().map(|q| q.number).collect();
            let range: Vec<u32> = block.items.iter().map(|i| i.number).collect();
            let (lo, hi) = (numbers.first().copied().unwrap_or(0), numbers.last().copied().unwrap_or(0));
            assert!(numbers.windows(2).all(|w| w[1] == w[0] + 1) && qs.iter().all(|q| q.options.len() == 5), "grupo {g} {}: {numbers:?}", block.subject);
            assert!(range.iter().all(|n| (lo..=hi).contains(n)) && numbers.len() == 15, "grupo {g} {}: key {range:?} vs exam {numbers:?}", block.subject);
            println!("grupo {g}: exam heading {} holds questions {lo}-{hi} ({} with 5 propositions) = key block {}", heads[b].1, numbers.len(), block.label);
        }
    }
    // Conhecimentos Gerais: 1-55 with 4 propositions, then Inglês, then Espanhol (56-60 each).
    let cg = text(&format!("{dir}/uepg-2025-conhecimentos-gerais.pdf"));
    let cg_lines: Vec<&str> = cg.split('\n').collect();
    let at = |name: &str| cg_lines.iter().position(|l| l.trim() == name).unwrap_or_else(|| panic!("{name} heading"));
    let (en, es) = (at("INGLÊS"), at("ESPANHOL"));
    assert!(en < es, "the exam gives Inglês before Espanhol");
    let part = |a: usize, b: usize| exam::split_questions(&cg_lines[a..b].join("\n"));
    for (name, qs, want) in [("before INGLÊS", part(0, en), (1..=55).collect::<Vec<u32>>()), ("INGLÊS", part(en, es), (56..=60).collect()), ("ESPANHOL", part(es, cg_lines.len()), (56..=60).collect())] {
        let numbers: Vec<u32> = qs.iter().map(|q| q.number).collect();
        assert!(numbers == want && qs.iter().all(|q| q.options.len() == 4), "{name}: {numbers:?}");
        println!("cg {name}: questions {}-{} with 4 propositions", want[0], want[want.len() - 1]);
    }
}
