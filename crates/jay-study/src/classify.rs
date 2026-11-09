//! Content or exam? Cheap rules, no model (docs/ARQUITETURA.md, section 1, "must check"): runs of
//! choices `a)`…`d)` in order, runs of sum-type propositions `01`, `02`, `04`…, question headers and
//! exam words. How dense the questions are, and where the first one starts, separates an exam from a
//! handout with exercises at the end. Jay-AI settles the unsure ones (stage 2); the student can always
//! change it.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaterialKind {
    Content,
    Exam,
    /// Content with exercises (a handout, a book chapter with a question list).
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    pub kind: MaterialKind,
    /// False when the rules couldn't tell well: the page asks the student to check.
    pub sure: bool,
    /// Multiple-choice questions found (runs of choices).
    pub choice_questions: u32,
    /// Sum-type questions found (runs of 01, 02, 04…).
    pub sum_questions: u32,
    /// "Questão 12", "Exercício 3"… headers.
    pub headers: u32,
}

/// Lowercase without Portuguese accents, for matching words.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' => 'e',
            'í' | 'î' | 'ì' | 'ï' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' | 'ö' => 'o',
            'ú' | 'û' | 'ù' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// The choice letter a line starts with: `a)`, `(a)`, `a.`, `a -`, `A)`, or ENEM's `A ` (the
/// letter in a circle, read as a lone capital before the text).
pub fn choice_letter(line: &str) -> Option<char> {
    let t = line.trim_start();
    let t = t.strip_prefix('(').unwrap_or(t);
    let mut chars = t.chars();
    let letter = chars.next()?;
    if !matches!(letter.to_ascii_lowercase(), 'a'..='e') {
        return None;
    }
    let rest = chars.as_str();
    let after = rest.trim_start();
    let marked = after.starts_with([')', '.', '-', '–', '—']) && !after.starts_with("..");
    // "A Curitiba": a capital, one space, then text (ENEM). Not "a casa" (an article in prose).
    let enem = letter.is_ascii_uppercase() && rest.starts_with(' ') && after.chars().next().is_some_and(|c| !c.is_ascii_punctuation());
    let rest_ok = |s: &str| s.trim_start_matches([')', '.', '-', '–', '—']).trim().chars().count() >= 1;
    ((marked && rest_ok(after)) || (enem && after.len() >= 2)).then_some(letter.to_ascii_lowercase())
}

/// The sum-type proposition a line starts with: `01)`, `02.`, `04 -`, `08 –`, `16)`, `32`, `64`.
pub fn sum_proposition(line: &str) -> Option<u32> {
    let t = line.trim_start();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() != 2 {
        return None;
    }
    let n: u32 = digits.parse().ok()?;
    let after = t[2..].trim_start();
    let marked = after.starts_with([')', '.', '-', '–', '—']) || after.chars().next().is_some_and(char::is_alphabetic);
    ([1, 2, 4, 8, 16, 32, 64].contains(&n) && marked && after.trim_start_matches([')', '.', '-', '–', '—']).trim().chars().count() >= 3).then_some(n)
}

/// "Questão 3", "QUESTÃO 03", "Exercício 2", "Question 4".
pub fn is_question_header(line: &str) -> bool {
    let f = fold(line.trim());
    ["questao", "exercicio", "question", "problema"].iter().any(|w| {
        f.strip_prefix(w).is_some_and(|rest| {
            let rest = rest.trim_start_matches([' ', '\u{a0}', 'n', 'º', '°', '.', ':']);
            rest.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
    })
}

/// Where each run of choices or propositions starts (line index), and what kind it is.
fn runs(lines: &[&str]) -> (Vec<usize>, Vec<usize>) {
    let mut choice_runs = vec![];
    let mut sum_runs = vec![];
    // Choices: a, b, c, d in order, each within 12 lines of the one before.
    let (mut expect, mut last_line, mut start) = ('a', 0usize, 0usize);
    for (i, line) in lines.iter().enumerate() {
        if let Some(letter) = choice_letter(line) {
            if letter == 'a' {
                (expect, start) = ('b', i);
            } else if letter == expect && i - last_line <= 12 {
                expect = (expect as u8 + 1) as char;
                if expect == 'e' {
                    choice_runs.push(start);
                }
            } else {
                expect = 'a';
            }
            last_line = i;
        }
    }
    // Propositions: 01, 02, 04 in order, each within 12 lines.
    let (mut expect, mut last_line, mut start) = (1u32, 0usize, 0usize);
    for (i, line) in lines.iter().enumerate() {
        if let Some(n) = sum_proposition(line) {
            if n == 1 {
                (expect, start) = (2, i);
            } else if n == expect && i - last_line <= 12 {
                expect *= 2;
                if expect == 8 {
                    sum_runs.push(start);
                }
            } else {
                expect = 1;
            }
            last_line = i;
        }
    }
    (choice_runs, sum_runs)
}

pub fn classify(text: &str) -> Classification {
    let lines: Vec<&str> = text.split(['\n', '\x0c']).collect();
    let (choice_runs, sum_runs) = runs(&lines);
    let headers = lines.iter().filter(|l| is_question_header(l)).count() as u32;
    let questions = (choice_runs.len() + sum_runs.len()) as u32;
    let folded = fold(text);
    let exam_words = ["gabarito", "assinale", "alternativa correta", "somatoria", "soma das", "marque a", "proposicoes corretas", "assinale a alternativa"].iter().filter(|w| folded.contains(*w)).count();
    let chars = text.chars().filter(|c| !c.is_whitespace()).count().max(1) as f32;
    // Questions per 1 000 letters: an exam has ~0.5 (ENEM's long base texts) or more; a handout
    // with a list of exercises at the end, ~0.1.
    let density = questions as f32 / (chars / 1000.0);
    let first_line = choice_runs.iter().chain(&sum_runs).min().copied().unwrap_or(lines.len());
    let first_at = lines[..first_line.min(lines.len())].iter().map(|l| l.chars().filter(|c| !c.is_whitespace()).count()).sum::<usize>() as f32 / chars;
    let (kind, sure) = match questions {
        0 if headers >= 3 && exam_words >= 1 => (MaterialKind::Exam, false),
        0 => (MaterialKind::Content, headers == 0),
        1..=2 => (if first_at > 0.5 { MaterialKind::Both } else { MaterialKind::Content }, false),
        _ if density >= 0.2 && first_at < 0.35 => (MaterialKind::Exam, density >= 0.3 || exam_words >= 1),
        _ => (MaterialKind::Both, first_at > 0.3),
    };
    Classification { kind, sure, choice_questions: choice_runs.len() as u32, sum_questions: sum_runs.len() as u32, headers }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prose(words: usize) -> String {
        let s = "A fotossíntese é o processo pelo qual as plantas transformam luz em energia química, liberando oxigênio. ";
        s.repeat(words / 16 + 1)
    }

    fn enem_question(n: u32) -> String {
        format!(
            "QUESTÃO {n:02}\nTexto-base curto sobre a fotossíntese e o ciclo do carbono, com um dado do IBGE.\nCom base no texto, a fotossíntese:\nA libera gás carbônico.\nB consome oxigênio.\nC produz glicose.\nD ocorre só à noite.\nE dispensa a luz.\n"
        )
    }

    fn uepg_question(n: u32) -> String {
        format!(
            "{n:02} – Sobre a fotossíntese, assinale o que for correto.\n01) Ocorre nos cloroplastos.\n02) Libera oxigênio.\n04) Consome glicose.\n08) Depende da luz.\n16) Ocorre nas mitocôndrias.\n"
        )
    }

    #[test]
    fn line_markers() {
        assert_eq!(choice_letter("a) Curitiba"), Some('a'));
        assert_eq!(choice_letter("(B) Londrina"), Some('b'));
        assert_eq!(choice_letter("  c. Maringá"), Some('c'));
        assert_eq!(choice_letter("D – Cascavel"), Some('d'));
        assert_eq!(choice_letter("E dispensa a luz."), Some('e'));
        assert_eq!(choice_letter("a casa é bonita"), None);
        assert_eq!(choice_letter("A"), None);
        assert_eq!(choice_letter("F) não"), None);
        assert_eq!(sum_proposition("01) Ocorre nos cloroplastos."), Some(1));
        assert_eq!(sum_proposition("08 – Depende da luz."), Some(8));
        assert_eq!(sum_proposition("16. Ocorre nas mitocôndrias"), Some(16));
        assert_eq!(sum_proposition("03) não é potência de 2"), None);
        assert_eq!(sum_proposition("2024 foi um ano"), None);
        assert!(is_question_header("QUESTÃO 03"));
        assert!(is_question_header("Questão nº 12"));
        assert!(is_question_header("Exercício 4:"));
        assert!(!is_question_header("Questões sociais do século XX"));
    }

    #[test]
    fn plain_content() {
        let c = classify(&prose(3000));
        assert_eq!((c.kind, c.sure), (MaterialKind::Content, true));
    }

    #[test]
    fn enem_exam() {
        let text: String = (1..=20).map(enem_question).collect();
        let c = classify(&text);
        assert_eq!((c.kind, c.sure, c.choice_questions), (MaterialKind::Exam, true, 20));
    }

    #[test]
    fn uepg_sum_exam() {
        let text: String = (1..=15).map(uepg_question).collect();
        let c = classify(&text);
        assert_eq!((c.kind, c.sure, c.sum_questions), (MaterialKind::Exam, true, 15));
    }

    #[test]
    fn handout_with_exercises_at_the_end() {
        let text = format!("{}\nExercícios\n{}", prose(8000), (1..=5).map(enem_question).collect::<String>());
        let c = classify(&text);
        assert_eq!((c.kind, c.sure), (MaterialKind::Both, true));
    }

    #[test]
    fn one_question_in_a_text_is_unsure() {
        let text = format!("{}\n{}", enem_question(1), prose(3000));
        let c = classify(&text);
        assert!(!c.sure);
    }
}
