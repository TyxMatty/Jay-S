//! Exams (stage 3): questions split out of an exam's text by rules (never by the model, so a reused
//! question comes back exactly as it was), answer keys, the styles, and grading.
//!
//! Splitting is a small state machine over the lines: a question starts at a header ("QUESTÃO 06")
//! or a numbered stem ("16– Sabendo…", "3. Qual…"); its options are runs of `a)`…`e)` or of
//! sum-type propositions `01)`…`16)`. Page furniture (ENEM's footers, "ESPAÇO RESERVADO PARA
//! ANOTAÇÕES", page numbers) is dropped first, and a shared base text ("TEXTO PARA AS QUESTÕES 01 A
//! 05") goes to every question it names.

use crate::classify::{choice_letter, fold, is_question_header, sum_proposition};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionKind {
    /// One choice among A–E.
    Choice,
    /// Sum-type: propositions 01, 02, 04…; the answer is the sum of the true ones.
    Sum,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QOption {
    /// "A"…"E", or "01", "02", "04"…
    pub label: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Question {
    /// The number in its exam (reused) or its position (new).
    pub number: u32,
    pub kind: QuestionKind,
    /// A base text shared with other questions ("TEXTO PARA AS QUESTÕES 01 A 05"), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passage: Option<String>,
    pub stem: String,
    pub options: Vec<QOption>,
    /// Choice: the letter. Sum: the sum ("21"). `None` when no key is known.
    #[serde(default)]
    pub answer: Option<String>,
    /// The answer came from Jay-AI, not an official key: the page says so.
    #[serde(default)]
    pub answer_by_ai: bool,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub explanation: Option<String>,
    /// Where it came from: the material id and, for a reused question, its number there.
    #[serde(default)]
    pub source: Option<Source>,
    /// Annulled by the board: shown, but not graded and out of the total.
    #[serde(default, skip_serializing_if = "is_false")]
    pub annulled: bool,
    /// Sum questions: the annulled propositions as a bitmask (0 = none). They are ignored in grading.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub void: u32,
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub material: String,
    /// `Some` for a question taken from an exam; `None` for one Jay-AI wrote from content.
    pub number: Option<u32>,
}

/// The most lines one option may take (measured: a UEPG literature proposition runs 14).
const MAX_OPTION_LINES: usize = 25;

/// Lines that are page furniture, not part of any question.
fn is_noise(line: &str) -> bool {
    let t = line.trim();
    let f = fold(t);
    t.is_empty()
        || (t.starts_with('*') && t.ends_with('*') && t.len() < 24)
        || (f.contains("caderno") && f.contains("dia"))
        || f.contains("espaco reservado")
        || f.starts_with("rascunho")
        || (t.len() <= 3 && t.chars().all(|c| c.is_ascii_digit()))
        || (f.starts_with("questoes de ") && t.len() < 30)
}

/// A line without its digits and spaces: page headers and footers differ only in the page number.
fn furniture_key(line: &str) -> String {
    fold(line).chars().filter(|c| c.is_alphabetic()).collect()
}

/// Headers and footers: short lines among the first or last three of a page that repeat on three
/// pages or more ("UEM/CVU – Vestibular de Verão 2023", "Caderno de Prova"). Only page edges are
/// looked at: "assinale o que for correto." repeats too, but inside the pages.
fn page_furniture(text: &str) -> std::collections::HashSet<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for page in text.split('\x0c') {
        let lines: Vec<&str> = page.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let edges = lines.iter().take(3).chain(lines.iter().rev().take(3));
        let mut seen = std::collections::HashSet::new();
        // Sentences (ending in . ? : ;) are text, never furniture, even at a page's edge.
        // Questions and options aren't furniture either: "QUESTÃO 06" opens many pages, and without
        // its digits it would look like a repeated header.
        let content = |l: &str| is_question_header(l) || numbered_stem(l).is_some() || choice_letter(l).is_some() || sum_proposition(l).is_some();
        for line in edges.filter(|l| l.chars().count() < 90 && !l.ends_with(['.', '?', ':', ';', ',']) && !content(l)) {
            let key = furniture_key(line);
            if key.len() >= 4 && seen.insert(key.clone()) {
                *counts.entry(key).or_default() += 1;
            }
        }
    }
    counts.into_iter().filter(|(_, n)| *n >= 3).map(|(k, _)| k).collect()
}

/// The lines of an exam, cleaned: tabs as spaces, page furniture out, and ENEM's doubled letters
/// ("A" alone on a line, then "A texto") as one line.
fn clean_lines(text: &str) -> Vec<String> {
    let furniture = page_furniture(text);
    let raw: Vec<String> = text.split(['\n', '\x0c']).map(|l| l.replace('\t', " ").split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    let mut out: Vec<String> = Vec::with_capacity(raw.len());
    for (i, line) in raw.iter().enumerate() {
        if is_noise(line) || (line.chars().count() < 90 && furniture.contains(&furniture_key(line))) {
            continue;
        }
        let lone = line.len() == 1 && matches!(line.as_bytes()[0], b'A'..=b'E');
        if lone && raw.get(i + 1).is_some_and(|next| next.starts_with(&format!("{line} "))) {
            continue;
        }
        out.push(line.clone());
    }
    out
}

/// A numbered stem: "16– Sabendo…", "3. Qual…", "12) Assinale…" (not a sum proposition, which the
/// caller rules out). Returns the number and the text after it.
fn numbered_stem(line: &str) -> Option<(u32, &str)> {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 3 {
        return None;
    }
    let rest = &line[digits.len()..];
    let mark = rest.trim_start();
    let after = mark.strip_prefix(['–', '—', '-', '.', ')'])?.trim_start();
    // "2.500 pessoas", "3,5": numbers inside prose aren't stems.
    if after.is_empty() || after.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some((digits.parse().ok()?, after))
}

/// "TEXTO PARA AS QUESTÕES 01 A 05" → (1, 5); "TEXTO PARA A QUESTÃO 7" → (7, 7).
fn passage_range(line: &str) -> Option<(u32, u32)> {
    let f = fold(line);
    if !(f.starts_with("texto para") || f.starts_with("textos para") || f.starts_with("leia o texto") || f.starts_with("o texto a seguir")) {
        return None;
    }
    let numbers: Vec<u32> = f.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect();
    match numbers.as_slice() {
        [a] => Some((*a, *a)),
        [a, b, ..] if b >= a => Some((*a, *b)),
        _ => Some((0, 0)),
    }
}

#[derive(Default)]
struct Draft {
    number: u32,
    kind: Option<QuestionKind>,
    stem: Vec<String>,
    options: Vec<QOption>,
}

impl Draft {
    fn finish(self) -> Option<Question> {
        let kind = self.kind?;
        let enough = match kind {
            QuestionKind::Choice => self.options.len() >= 4,
            QuestionKind::Sum => self.options.len() >= 3,
        };
        let stem = self.stem.join(" ").trim().to_owned();
        (enough && !stem.is_empty()).then(|| Question {
            number: self.number,
            kind,
            passage: None,
            stem,
            options: self.options,
            answer: None,
            answer_by_ai: false,
            topic: None,
            explanation: None,
            source: None,
            annulled: false,
            void: 0,
        })
    }

    /// Whether `label` (of a confirmed run) is this question's next option.
    fn takes(&self, kind: QuestionKind, label: &str) -> bool {
        let n = self.options.len();
        self.kind.is_none_or(|k| k == kind)
            && match kind {
                QuestionKind::Choice => label.as_bytes()[0] == b'A' + n as u8,
                QuestionKind::Sum => label.parse::<u32>().is_ok_and(|v| v == 1 << n),
            }
    }
}

/// The mark right after an option's label: ')' in "a)" and "01)", '–' in "08 –", ' ' in ENEM's
/// "A texto". Every option of a run uses the same one, so UEPG's question "32– Prática…" right after
/// a proposition "16)" isn't taken for proposition 32.
fn label_mark(line: &str) -> char {
    let t = line.trim_start();
    let t = t.strip_prefix('(').unwrap_or(t);
    let rest = t.trim_start_matches(|c: char| c.is_ascii_alphanumeric());
    rest.trim_start().chars().next().filter(|c| !c.is_alphanumeric()).unwrap_or(' ')
}

/// For each line, its option label when it belongs to a confirmed run: choices a→b→c→d (at least)
/// or propositions 01→02→04 (at least), in order, with the same mark after the label, each within
/// `MAX_OPTION_LINES` of the one before. Alone, a line like "A sessão do Comitê…" (ENEM prose) or
/// "01– Sobre o texto…" (a UEPG question) only looks like an option.
fn option_labels(lines: &[String]) -> Vec<Option<(QuestionKind, String)>> {
    let mut labels = vec![None; lines.len()];
    let mut confirm = |run: &mut Vec<(usize, String)>, kind: QuestionKind, min: usize| {
        if run.len() >= min {
            for (i, label) in run.drain(..) {
                labels[i] = Some((kind, label));
            }
        }
        run.clear();
    };
    let mut run: Vec<(usize, String)> = vec![];
    for (i, line) in lines.iter().enumerate() {
        let Some(letter) = choice_letter(line) else { continue };
        let expected = (b'a' + run.len() as u8) as char;
        let close_enough = run.last().is_none_or(|(j, _)| i - j <= MAX_OPTION_LINES);
        let same_mark = run.first().is_none_or(|(j, _)| label_mark(&lines[*j]) == label_mark(line));
        if letter == expected && close_enough && same_mark && !run.is_empty() {
            run.push((i, letter.to_ascii_uppercase().to_string()));
        } else {
            confirm(&mut run, QuestionKind::Choice, 4);
            if letter == 'a' {
                run.push((i, "A".into()));
            }
        }
    }
    confirm(&mut run, QuestionKind::Choice, 4);
    for (i, line) in lines.iter().enumerate() {
        let Some(n) = sum_proposition(line) else { continue };
        let close_enough = run.last().is_none_or(|(j, _)| i - j <= MAX_OPTION_LINES);
        let same_mark = run.first().is_none_or(|(j, _)| label_mark(&lines[*j]) == label_mark(line));
        if n == 1 << run.len() && close_enough && same_mark && !run.is_empty() {
            run.push((i, format!("{n:02}")));
        } else {
            confirm(&mut run, QuestionKind::Sum, 3);
            if n == 1 {
                run.push((i, "01".into()));
            }
        }
    }
    confirm(&mut run, QuestionKind::Sum, 3);
    labels
}

/// Strips the option's own label ("a)", "(B)", "A ", "01)", "08 –") from its line.
fn option_text(line: &str) -> String {
    let t = line.trim_start();
    let t = t.strip_prefix('(').unwrap_or(t);
    let skip = t.chars().take_while(|c| c.is_ascii_alphanumeric()).count();
    t[skip..].trim_start().trim_start_matches([')', '.', '-', '–', '—']).trim().to_owned()
}

/// The questions of an exam's text, in order. Lines that aren't part of a recognized question (cover
/// pages, instructions) are left out.
pub fn split_questions(text: &str) -> Vec<Question> {
    let lines = clean_lines(text);
    let labels = option_labels(&lines);
    let mut out: Vec<Question> = vec![];
    let mut draft: Option<Draft> = None;
    // A base text waiting for the questions it names.
    let mut passage: Option<((u32, u32), Vec<String>)> = None;
    // With the index (in `out`) of the question that comes right after each.
    let mut passages: Vec<((u32, u32), String, usize)> = vec![];
    // After the last option, a few continuation lines at most; then text goes to the next question.
    let mut after_options = 0usize;
    let close = |draft: &mut Option<Draft>, out: &mut Vec<Question>| {
        if let Some(q) = draft.take().and_then(Draft::finish) {
            out.push(q);
        }
    };
    for (i, line) in lines.iter().enumerate() {
        if let Some(range) = passage_range(line) {
            close(&mut draft, &mut out);
            if let Some((r, body)) = passage.take() {
                passages.push((r, body.join(" "), out.len()));
            }
            passage = Some((range, vec![]));
            continue;
        }
        let in_options = draft.as_ref().is_some_and(|d| !d.options.is_empty());
        if let Some(d) = draft.as_mut()
            && let Some((kind, label)) = &labels[i]
            && !d.stem.is_empty()
            && d.takes(*kind, label)
        {
            d.kind = Some(*kind);
            d.options.push(QOption { label: label.clone(), text: option_text(line) });
            after_options = 0;
            continue;
        }
        let header = is_question_header(line).then(|| line.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse::<u32>().ok()).flatten();
        // While this question's next option is still ahead, a line like "38) es una palabra…" (a
        // wrapped "(línea 38)") continues the option instead of starting question 38.
        let option_pending = in_options
            && draft.as_ref().is_some_and(|d| {
                labels[i + 1..].iter().take(MAX_OPTION_LINES).flatten().next().is_some_and(|(kind, label)| d.takes(*kind, label))
            });
        let stem_start = if header.is_some() || labels[i].is_some() || option_pending { None } else { numbered_stem(line) };
        if header.is_some() || stem_start.is_some() {
            close(&mut draft, &mut out);
            if let Some((r, body)) = passage.take() {
                passages.push((r, body.join(" "), out.len()));
            }
            let (number, rest) = match (header, stem_start) {
                (Some(n), _) => (n, None),
                (_, Some((n, rest))) => (n, Some(rest.to_owned())),
                _ => unreachable!(),
            };
            draft = Some(Draft { number, stem: rest.into_iter().collect(), ..Default::default() });
            after_options = 0;
            continue;
        }
        match draft.as_mut() {
            // Continuation of the option (wrapped lines; UEPG's run to 6). It ends at the next
            // question, base text or section title, or after MAX_OPTION_LINES lines.
            Some(d) if in_options => {
                after_options += 1;
                let all_caps = line.chars().any(char::is_alphabetic) && line.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase) && line.len() > 6;
                if after_options <= MAX_OPTION_LINES && !all_caps {
                    let last = d.options.last_mut().unwrap();
                    last.text.push(' ');
                    last.text.push_str(line);
                } else {
                    close(&mut draft, &mut out);
                }
            }
            Some(d) => d.stem.push(line.clone()),
            None => {
                if let Some((_, body)) = passage.as_mut() {
                    body.push(line.clone());
                }
            }
        }
    }
    close(&mut draft, &mut out);
    if let Some((r, body)) = passage.take() {
        passages.push((r, body.join(" "), out.len()));
    }
    // Instructions can hold question-shaped examples ("Suponha a questão hipotética número 65", UEM's
    // "Terá nota ZERO… A) … E)"); they come before the exam's own question 1, and go.
    if let Some(first) = out.iter().position(|q| q.number == 1) {
        out.drain(..first);
        passages.retain_mut(|p| p.2.checked_sub(first).map(|next| p.2 = next).is_some());
    }
    // ENEM-like stems carry their own base text: keep it apart from the command, which is what's
    // asked (and shown in bold).
    for q in out.iter_mut().filter(|q| q.passage.is_none()) {
        if let Some((passage, command)) = split_command(&q.stem) {
            q.passage = Some(passage);
            q.stem = command;
        }
    }
    // Each base text goes to the questions in its range (or, without one, to the next question).
    for ((a, b), body, next) in passages.into_iter().filter(|p| !p.1.trim().is_empty()) {
        if a == 0 {
            if let Some(q) = out.get_mut(next) {
                q.passage.get_or_insert(body);
            }
            continue;
        }
        for q in out.iter_mut().filter(|q| (a..=b).contains(&q.number)) {
            q.passage.get_or_insert_with(|| body.clone());
        }
    }
    out
}

/// A long stem as (base text, command): the command starts after the last source citation
/// ("(adaptado).", "Acesso em: …."), or else is the last sentence. Short stems stay whole.
fn split_command(stem: &str) -> Option<(String, String)> {
    if stem.chars().count() < 280 {
        return None;
    }
    let cut = ["(adaptado).", "(adaptado)", "(fragmento).", "Acesso em:"]
        .iter()
        .filter_map(|m| stem.rfind(m).map(|p| p + m.len()))
        .max()
        .and_then(|after| {
            // The citation's own sentence ends at the next ". " (the date: "8 nov. 2022.") or right here.
            let rest = &stem[after..];
            if rest.trim_start().chars().next().is_some_and(char::is_uppercase) {
                Some(after)
            } else {
                rest.match_indices(". ").map(|(i, _)| after + i + 1).find(|&i| stem[i..].trim_start().chars().next().is_some_and(char::is_uppercase))
            }
        })
        .or_else(|| stem.rfind(". ").map(|i| i + 1))?;
    let (passage, command) = (stem[..cut].trim(), stem[cut..].trim());
    (command.chars().count() >= 15 && command.chars().count() <= 600 && passage.chars().count() >= 80).then(|| (passage.to_owned(), command.to_owned()))
}

/// The sum of a sum-type answer's true propositions ("01", "04", "16" → 21). `None` unless each one
/// is a different power of two, written with two digits.
fn propositions<'a>(parts: impl Iterator<Item = &'a str>) -> Option<u32> {
    let mut sum = 0;
    for part in parts {
        let n = two_digits(part).filter(|n| n.is_power_of_two() && sum & n == 0)?;
        sum |= n;
    }
    (sum > 0).then_some(sum)
}

fn two_digits(s: &str) -> Option<u32> {
    (s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

/// A sum-type key that lists each answer's true propositions, so every answer proves itself (19 is
/// 01 + 02 + 16). UEM: "01 19 01-02-16" (question, sum, propositions), or "19 01-02-16" when the
/// table left the PDF out of order and the question's number went elsewhere. UEPG: three lines, the
/// questions' numbers, the propositions of each ("01 02 - - 16") and the sums. A key is at least 8
/// proven answers that make up a third of the text or more (an exam with its key at the end isn't one).
///
/// These files often hold several exams, each counting from 1 again, and three foreign languages
/// under the same numbers. Nothing here says which one the student's exam is, so a number answered in
/// two different ways gets no answer at all: a missing answer goes to Jay-AI, a wrong one would grade
/// the student wrongly.
/// shortcut: a key of several exams is recognized but answers little, upgrade by matching each
/// block's heading to the exam when students attach such keys.
fn parse_sum_key(text: &str) -> Option<BTreeMap<u32, Vec<String>>> {
    let lines: Vec<Vec<&str>> = text.split(['\n', '\x0c']).map(|l| l.split_whitespace().collect::<Vec<_>>()).filter(|l| !l.is_empty()).collect();
    let size = |line: &[&str]| line.iter().map(|t| t.len()).sum::<usize>();
    let numbers = |line: &[&str]| line.iter().map(|t| two_digits(t)).collect::<Option<Vec<u32>>>();
    let mut found: Vec<(Option<u32>, u32)> = vec![];
    let mut proven = 0;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].as_slice();
        i += 1;
        if let (Some(questions), Some(marks), Some(sums)) = (numbers(line), lines.get(i), lines.get(i + 1).and_then(|l| numbers(l))) {
            let each = marks.len() / line.len();
            if sums.len() == questions.len() && marks.len() % line.len() == 0 && (3..=7).contains(&each) {
                let block: Vec<_> = questions
                    .iter()
                    .zip(marks.chunks(each))
                    .zip(&sums)
                    .filter(|((_, marks), sum)| propositions(marks.iter().copied().filter(|m| *m != "-")) == Some(**sum))
                    .map(|((question, _), sum)| (Some(*question), *sum))
                    .collect();
                if !block.is_empty() {
                    found.extend(block);
                    proven += size(line) + size(marks) + size(&lines[i + 1]);
                    i += 2;
                    continue;
                }
            }
        }
        let (question, sum, parts) = match line {
            [question, sum, parts] => match two_digits(question).filter(|q| *q > 0) {
                Some(question) => (Some(question), sum, parts),
                None => continue,
            },
            [sum, parts] => (None, sum, parts),
            _ => continue,
        };
        // "16*": UEM marks a corrected answer.
        if let Some(sum) = two_digits(sum.trim_end_matches('*')).filter(|sum| propositions(parts.split('-')) == Some(*sum)) {
            found.push((question, sum));
            proven += size(line);
        }
    }
    if found.len() < 8 || proven * 3 < lines.iter().map(|l| size(l)).sum::<usize>() {
        return None;
    }
    let mut answers: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for (question, sum) in found.into_iter().filter_map(|(q, s)| Some((q?, s))) {
        answers.entry(question).and_modify(|a| *a = a.filter(|a| *a == sum)).or_insert(Some(sum));
    }
    Some(answers.into_iter().filter_map(|(q, a)| Some((q, vec![format!("{:02}", a?)]))).collect())
}

/// An answer key's text ("46 C", "1 B A" for ENEM's two languages, "17 21" for sum-type, and the
/// sum-type layouts of `parse_sum_key`): question number → the answers, in order. `None` when the
/// text doesn't look like a key. A key can come out with few answers, or none: only the ones the text
/// ties to one question are here.
pub fn parse_answer_key(text: &str) -> Option<BTreeMap<u32, Vec<String>>> {
    if let Some(key) = parse_sum_key(text) {
        return Some(key);
    }
    let mut key: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut lines = 0usize;
    for line in text.split(['\n', '\x0c']).map(str::trim).filter(|l| !l.is_empty()) {
        lines += 1;
        let parts: Vec<&str> = line.split_whitespace().collect();
        let Some((first, rest)) = parts.split_first() else { continue };
        let Ok(number) = first.parse::<u32>() else { continue };
        let answer = |s: &&str| (s.len() == 1 && matches!(s.as_bytes()[0], b'A'..=b'E')) || (s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit())) || fold(s) == "anulada" || *s == "*";
        if (1..=3).contains(&rest.len()) && rest.iter().all(answer) && number > 0 && number < 1000 {
            key.insert(number, rest.iter().map(|s| s.to_string()).collect());
        }
    }
    (key.len() >= 8 && key.len() * 2 >= lines.min(key.len() * 3)).then_some(key)
}

/// Puts an answer key on split questions. A number that appears twice in the exam (ENEM's questions
/// 1–5 in English, then in Spanish) takes the key's answers in the same order. When the exam and the
/// key don't have a number the same number of times (a misread number, a UEM exam with three
/// languages), none of those questions gets an answer: which answer is whose would be a guess.
pub fn apply_answer_key(questions: &mut [Question], key: &BTreeMap<u32, Vec<String>>) -> usize {
    let mut times: BTreeMap<u32, usize> = BTreeMap::new();
    for q in questions.iter() {
        *times.entry(q.number).or_default() += 1;
    }
    let mut seen: BTreeMap<u32, usize> = BTreeMap::new();
    let mut applied = 0;
    for q in questions.iter_mut() {
        let nth = seen.entry(q.number).or_default();
        if let Some(answer) = key.get(&q.number).filter(|a| a.len() == times[&q.number]).and_then(|a| a.get(*nth)) {
            let valid = match q.kind {
                QuestionKind::Choice => q.options.iter().any(|o| &o.label == answer),
                QuestionKind::Sum => answer.parse::<u32>().is_ok_and(|s| s > 0 && s < (1 << q.options.len())),
            };
            if valid {
                q.answer = Some(answer.clone());
                q.answer_by_ai = false;
                applied += 1;
            }
        }
        *nth += 1;
    }
    applied
}

/// How a question is graded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scoring {
    /// One right choice: all or nothing.
    Single,
    /// Sum-type with partial credit (UEPG, 2025 exam instructions): any wrong proposition marked →
    /// zero; otherwise the share of the true ones that were marked.
    SumPartial,
    /// Sum-type, only the exact sum counts.
    SumExact,
}

/// The points (0.0–1.0) of an answer. `marked`: the letter, or the sum as a number string.
pub fn grade(kind: QuestionKind, scoring: Scoring, answer: &str, marked: &str) -> f32 {
    match kind {
        QuestionKind::Choice => f32::from(answer.eq_ignore_ascii_case(marked.trim())),
        QuestionKind::Sum => {
            let (Ok(right), Ok(given)) = (answer.parse::<u32>(), marked.trim().parse::<u32>()) else { return 0.0 };
            match scoring {
                Scoring::SumExact | Scoring::Single => f32::from(right == given),
                Scoring::SumPartial => {
                    if given == 0 || given & !right != 0 {
                        0.0
                    } else {
                        (given & right).count_ones() as f32 / right.count_ones().max(1) as f32
                    }
                }
            }
        }
    }
}

/// An exam style (presets/*.json): how questions look, how many options, how they're graded, and
/// what Jay-AI is told when it writes one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub id: String,
    pub name: String,
    pub kind: QuestionKind,
    /// Choices (A–E = 5) or propositions (01–16 = 5).
    pub options: u32,
    pub scoring: Scoring,
    /// Shown under the name.
    pub description: String,
    /// For Jay-AI, in Portuguese (the exams are).
    pub prompt: String,
}

const PRESETS: [&str; 3] = [include_str!("../presets/enem.json"), include_str!("../presets/uepg.json"), include_str!("../presets/uem.json")];

/// The built-in styles, read and checked like a file someone could have edited.
pub fn styles() -> Vec<Style> {
    PRESETS.iter().filter_map(|s| serde_json::from_str::<Style>(s).ok()).filter(|s| (2..=7).contains(&s.options) && !s.id.is_empty()).collect()
}

pub fn style(id: &str) -> Option<Style> {
    styles().into_iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENEM: &str = "es metrópoles.\nE \nE falta de segurança existente no transporte público \nurbano.\n*010175AZ5*\x0c6 –LC • 1º DIA • CADERNO 1 • AZUL–\nLINGUAGENS, CÓDIGOS E SUAS TECNOLOGIAS\nQuestões de 06 a 45\nQUESTÃO 06 \nA sessão do Comitê Olímpico Internacional (COI) \naprovou uma mudança histórica no lema.\nDisponível\tem:\thttps://ge.globo.com.\tAcesso\tem:\t10\tnov.\t2021\t(adaptado).\nDe acordo com o texto, a alteração do lema olímpico teve \ncomo objetivo a\nA \nA unificação\tdo\tlema\tanterior\tao\tatual.\nB \nB aproximação entre o lema olímpico e o COI.\nC \nC junção do lema olímpico com os princípios esportivos.\nD \nD associação entre o lema olímpico e a cooperatividade.\nE \nE vinculação entre o lema olímpico e os eventos atléticos.\nQUESTÃO 07 \nMais iluminada que outras \nTenho dois seios, estas duas coxas.\nNo texto, a autora\nA \nA critica.\nB \nB elogia.\nC \nC narra.\nD \nD ironiza.\nE \nE descreve.\n";

    const UEPG: &str = "PORTUGUÊS \nTEXTO PARA AS QUESTÕES 01 A 02 \nVocê está limpando sua garrafa de água? \nCarl Behnke é especialista em segurança alimentar. \n01– Sobre o texto, assinale o que for correto. \n01) Fala de garrafas. \n02) Fala de Purdue. \n04) Fala de carros. \n08) É um texto científico. \n16) Cita um especialista. \n02– Assinale o que for correto. \n01) A. \n02) B é \nlonga. \n04) C. \n08) D. \n16) E. \nESPAÇO RESERVADO PARA ANOTAÇÕES \x0c7 \nMATEMÁTICA \n16– Sabendo que cada letra representa \num algarismo, assinale o que for \ncorreto. \n01) E = 8. \n02) G = 4. \n04) U = 7. \n08) P = 5. \n16)  U + E > P + G. \n";

    #[test]
    fn enem_questions_with_footers_and_doubled_letters() {
        let qs = split_questions(ENEM);
        assert_eq!(qs.len(), 2, "{qs:#?}");
        let q = &qs[0];
        assert_eq!((q.number, q.kind), (6, QuestionKind::Choice));
        assert!(q.stem.starts_with("A sessão do Comitê") && q.stem.ends_with("teve como objetivo a"), "{}", q.stem);
        assert!(q.stem.contains("Disponível em: https://ge.globo.com."), "{}", q.stem);
        assert_eq!(q.options.iter().map(|o| o.label.as_str()).collect::<String>(), "ABCDE");
        assert_eq!(q.options[0].text, "unificação do lema anterior ao atual.");
        assert_eq!(qs[1].options[4].text, "descreve.");
    }

    #[test]
    fn uepg_sum_questions_with_a_shared_text() {
        let qs = split_questions(UEPG);
        assert_eq!(qs.iter().map(|q| q.number).collect::<Vec<_>>(), [1, 2, 16], "{qs:#?}");
        assert!(qs.iter().all(|q| q.kind == QuestionKind::Sum && q.options.len() == 5));
        assert_eq!(qs[0].options.iter().map(|o| o.label.as_str()).collect::<Vec<_>>(), ["01", "02", "04", "08", "16"]);
        assert_eq!(qs[1].options[1].text, "B é longa.");
        // The shared base text goes to questions 01 and 02, not to 16.
        assert!(qs[0].passage.as_deref().unwrap().contains("Carl Behnke"));
        assert_eq!(qs[0].passage, qs[1].passage);
        assert_eq!(qs[2].passage, None);
        assert_eq!(qs[2].stem, "Sabendo que cada letra representa um algarismo, assinale o que for correto.");
        // "MATEMÁTICA" (a section title) didn't end up in question 02's last option.
        assert_eq!(qs[1].options[4].text, "E.");
    }

    #[test]
    fn a_base_text_that_names_no_question_goes_to_the_next_one() {
        let text = "1) Quanto é um mais um?\nA) Um.\nB) Dois.\nC) Três.\nD) Quatro.\nLeia o texto a seguir.\nO rio corre para o mar desde sempre.\n2) Segundo o texto, o rio corre para onde?\nA) O mar.\nB) O lago.\nC) A serra.\nD) O céu.\n3) Quanto é dois mais dois?\nA) Um.\nB) Dois.\nC) Três.\nD) Quatro.\n";
        let qs = split_questions(text);
        assert_eq!(qs.iter().map(|q| q.passage.as_deref()).collect::<Vec<_>>(), [None, Some("O rio corre para o mar desde sempre."), None], "{qs:#?}");
    }

    #[test]
    fn a_question_numbered_32_after_proposition_16_is_a_question() {
        let text = "31– Sobre Roma, assinale o que for correto. \n01) Um. \n02) Dois. \n04) Quatro. \n08) Oito. \n16) Dezesseis. \n32– Prática que teve início na colônia, assinale o que for correto. \n01) A. \n02) B. \n04) C. \n08) D. \n16) E. \n";
        let qs = split_questions(text);
        assert_eq!(qs.iter().map(|q| (q.number, q.options.len())).collect::<Vec<_>>(), [(31, 5), (32, 5)]);
    }

    #[test]
    fn enem_base_text_and_command_apart() {
        let stem = "TEXTO I Por que me custa tanto estudar? Disponível em: www.exemplo.org. Acesso em: 8 nov. 2022. TEXTO II Ishaan Awashi é um menino de 8 anos cujo mundo está cheio de maravilhas que ninguém parece apreciar. Disponível em: https://elfinalde.com. Acesso em: 26 out. 2021 (adaptado). O filme aborda o tema da dislexia. Relacionando o cartaz do filme com a sinopse, constata-se que o(a)";
        let (passage, command) = split_command(stem).unwrap();
        assert!(passage.ends_with("(adaptado)."), "{passage}");
        assert_eq!(command, "O filme aborda o tema da dislexia. Relacionando o cartaz do filme com a sinopse, constata-se que o(a)");
        let plain = "A sessão do Comitê Olímpico Internacional aprovou uma mudança histórica no lema olímpico, criado em 1894 pelo Barão Pierre de Coubertin para expressar os valores do esporte. Mais de 120 anos depois, o lema tem sua primeira alteração para incluir a palavra juntos. De acordo com o texto, a alteração do lema teve como objetivo a";
        assert_eq!(split_command(plain).unwrap().1, "De acordo com o texto, a alteração do lema teve como objetivo a");
        assert!(split_command("Qual organela faz a fotossíntese?").is_none());
    }

    #[test]
    fn a_teachers_exam() {
        let text = "Prova de Biologia - 2º ano\nNome: ______\n1. Qual organela faz a fotossíntese?\na) Mitocôndria\nb) Cloroplasto\nc) Ribossomo\nd) Lisossomo\n2) Em 2.500 anos, quantas gerações? Marque a correta.\n(a) 10\n(b) 100\n(c) 1000\n(d) 25\n(e) 250\n";
        let qs = split_questions(text);
        assert_eq!(qs.len(), 2, "{qs:#?}");
        assert_eq!(qs[0].stem, "Qual organela faz a fotossíntese?");
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[1].stem, "Em 2.500 anos, quantas gerações? Marque a correta.");
        assert_eq!(qs[1].options[4].text, "250");
    }

    #[test]
    fn answer_keys() {
        let enem = "CADERNO 1\nAzul\nGabarito 2023\nQUESTÃO GABARITO\n46 C\n47 D\n48 A\n49 E\n50 E\n51 C\n52 A\n53 B\nINGLÊS ESPANHOL\n1 B A\n2 B A\n3 D E\n6 D\n7 A\n";
        let key = parse_answer_key(enem).unwrap();
        assert_eq!(key[&46], ["C"]);
        assert_eq!(key[&1], ["B", "A"]);
        let uepg = (1..=15).map(|n| format!("{n:02} {:02}\n", (n * 7) % 31 + 1)).collect::<String>();
        assert_eq!(parse_answer_key(&uepg).unwrap()[&2], ["15"]);
        // Prose with a few numbers isn't a key.
        assert!(parse_answer_key("A fotossíntese\n1 A planta\nocorre nos cloroplastos.\n2 B\n").is_none());
    }

    #[test]
    fn sum_keys_prove_each_answer_and_never_guess_the_question() {
        let answer = |key: &BTreeMap<u32, Vec<String>>, n: u32| key.get(&n).map(|a| a.join(","));
        // UEM: question, sum, true propositions. Three languages share 11 and 12; "16*" was corrected.
        let uem = "GABARITO DEFINITIVO\nQuestão Resposta Alternativa(s) Correta(s)\n01 19 01-02-16\n02 26 02-08-16\n03 11 01-02-08\n04 18 02-16\n05 31 01-02-04-08-16\n06 25 01-08-16\n07 04 04\n08 18 02-16\n09 16* 16\n10 10 02-08\nE\nS\nP\n11 18 02-16\n12 19 01-02-16\nF\nR\n11 19 01-02-16\n12 19 01-02-16\n";
        let key = parse_answer_key(uem).unwrap();
        assert_eq!(answer(&key, 1).as_deref(), Some("19"));
        assert_eq!(answer(&key, 7).as_deref(), Some("04"));
        assert_eq!(answer(&key, 9).as_deref(), Some("16"));
        // 11 has two answers (which language?): none. 12 has the same answer twice: that one.
        assert_eq!(answer(&key, 11), None);
        assert_eq!(answer(&key, 12).as_deref(), Some("19"));

        // UEM with the table out of order: it's a key, but no line says which question it answers
        // ("04 04" is the sum 4 and its proposition, not question 4).
        let loose = "Resposta Alternativa(s) Correta(s)\n04 04\n25 01-08-16\n21 01-04-16\n08 08\n10 02-08\n16 16\n15 01-02-04-08\n06 02-04\n16\n17\nQuestão\n05\n06\n";
        assert_eq!(parse_answer_key(loose).unwrap().len(), 0);

        // UEPG: the numbers, each question's propositions, the sums. Question 03 says 03 for "01 - 04 -"
        // (5): unproven, so no answer. A second exam in the same file counts from 01 again.
        let uepg = "Número da questão\nAlternativas corretas\nsomatório correto\n01 02 03 04 05 06 07\n01 - - 08 01 - 04 - 01 - 04 - 01 02 04 08 - - 04 08 01 02 04 - - 02 - 08\n09 05 03 15 12 07 10\n08 09\n01 02 04 08 01 02 - -\n15 03\nQUÍMICA\n01 02\n- 02 04 - 01 02 - -\n06 03\n";
        let key = parse_answer_key(uepg).unwrap();
        assert_eq!(answer(&key, 1), None);
        assert_eq!(answer(&key, 2), None);
        assert_eq!(answer(&key, 3), None);
        assert_eq!(answer(&key, 4).as_deref(), Some("15"));
        assert_eq!(answer(&key, 9).as_deref(), Some("03"));

        // Not keys: sums that don't add up, propositions that aren't powers of two or repeat, and an
        // exam with its key on the last page (the key is a small part of the text).
        let wrong = (1..=12).map(|n| format!("{n:02} 20 01-02-16\n")).collect::<String>();
        assert!(parse_sum_key(&wrong).is_none());
        let dates = (1..=12).map(|n| format!("{n:02} 21 03-06-12\n")).collect::<String>();
        assert!(parse_sum_key(&dates).is_none());
        let twice = (1..=12).map(|n| format!("{n:02} 04 02-02\n")).collect::<String>();
        assert!(parse_sum_key(&twice).is_none());
        let exam = format!("{}{uem}", "Assinale o que for correto sobre a fotossíntese e some as proposições verdadeiras.\n".repeat(12));
        assert!(parse_answer_key(&exam).is_none());
        // Seven proven answers aren't enough.
        assert!(parse_sum_key("01 19 01-02-16\n02 26 02-08-16\n03 11 01-02-08\n04 18 02-16\n05 31 01-02-04-08-16\n06 25 01-08-16\n07 04 04\n").is_none());
    }

    #[test]
    fn keys_go_to_the_right_questions_including_enems_two_languages() {
        let q = |n: u32| Question {
            number: n,
            kind: QuestionKind::Choice,
            passage: None,
            stem: "?".into(),
            options: "ABCDE".chars().map(|c| QOption { label: c.to_string(), text: "x".into() }).collect(),
            answer: None,
            answer_by_ai: false,
            topic: None,
            explanation: None,
            source: None,
            annulled: false,
            void: 0,
        };
        // Questions 1 (English), 1 (Spanish), 6.
        let mut qs = vec![q(1), q(1), q(6), q(7)];
        let key = BTreeMap::from([(1, vec!["B".to_string(), "A".to_string()]), (6, vec!["D".to_string()]), (7, vec!["Z".to_string()])]);
        assert_eq!(apply_answer_key(&mut qs, &key), 3);
        assert_eq!(qs.iter().map(|q| q.answer.clone()).collect::<Vec<_>>(), [Some("B".into()), Some("A".into()), Some("D".into()), None]);
        // A number twice in the exam and once in the key (or the other way round): whose answer is it?
        let mut qs = vec![q(1), q(6), q(6), q(8)];
        let key = BTreeMap::from([(1, vec!["B".to_string(), "A".to_string()]), (6, vec!["D".to_string()]), (8, vec!["C".to_string()])]);
        assert_eq!(apply_answer_key(&mut qs, &key), 1);
        assert_eq!(qs.iter().map(|q| q.answer.clone()).collect::<Vec<_>>(), [None, None, None, Some("C".into())]);
    }

    #[test]
    fn grading() {
        use QuestionKind::*;
        assert_eq!(grade(Choice, Scoring::Single, "C", "c"), 1.0);
        assert_eq!(grade(Choice, Scoring::Single, "C", "D"), 0.0);
        // Answer 21 = 01 + 04 + 16.
        assert_eq!(grade(Sum, Scoring::SumPartial, "21", "21"), 1.0);
        assert!((grade(Sum, Scoring::SumPartial, "21", "05") - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(grade(Sum, Scoring::SumPartial, "21", "23"), 0.0, "02 is wrong");
        assert_eq!(grade(Sum, Scoring::SumPartial, "21", "00"), 0.0);
        assert_eq!(grade(Sum, Scoring::SumExact, "21", "05"), 0.0);
        assert_eq!(grade(Sum, Scoring::SumExact, "21", "21"), 1.0);
        assert_eq!(grade(Sum, Scoring::SumPartial, "21", "abc"), 0.0);
    }

    #[test]
    fn built_in_styles_load() {
        let ids: Vec<String> = styles().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["enem", "uepg", "uem"]);
        assert_eq!(style("uepg").unwrap().scoring, Scoring::SumPartial);
    }
}
