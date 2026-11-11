//! The catalogue of UEPG's Vestibular 2025 exams (official files, with the official answers). The
//! sums come from the edital's key after appeals (`examples/uepg_key.rs` reads them and checks each
//! against its propositions); the exams' own files are downloaded, never shipped. An annulled
//! question has no sum (everyone gets the point); an annulled proposition (`void_options`) is left
//! out of the sum and the share of the others is recalculated (Res. UNIV 2023.15, art. 26).

use crate::ai::{AiError, Download, download};
use crate::exam::{self, Question, Source};
use crate::records::{BuildError, Exam};
use crate::store::{Store, StoreError, now_ms, read_json, write_json};
use crate::{extract, id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

/// The key's edital (Edital 51/2025-CPS, after appeals), kept for reference.
pub const KEY_URL: &str = "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/EDITAL-N-51_2025-_CPS_GABARITO-VESTIBULAR-2025_Apos-Recursos.pdf";
pub const KEY_SHA256: &str = "cbf51df58d7c9077109c581896526001ef165edcca15eb32fa412203402cf40e";

#[derive(Debug)]
pub struct Entry {
    pub id: &'static str,
    pub family: &'static str,
    pub year: u32,
    pub title: &'static str,
    /// An exam style (presets/*.json).
    pub style: &'static str,
    pub download: Download,
    /// Questions the exam splits into (after `keep_language`).
    pub questions: u32,
    /// Conhecimentos Gerais repeats questions 56-60 for each language, in the file's order: 0 =
    /// Inglês, 1 = Espanhol. `None` for the groups.
    pub language: Option<usize>,
    /// (number, sum as the key writes it).
    pub answers: &'static [(u32, &'static str)],
    pub annulled: &'static [u32],
    /// (number, proposition) annulled by the appeals.
    pub void_options: &'static [(u32, &'static str)],
}

impl Entry {
    /// The answers as `exam::apply_answer_key` takes them.
    pub fn answer_key(&self) -> BTreeMap<u32, Vec<String>> {
        self.answers.iter().map(|(n, s)| (*n, vec![s.to_string()])).collect()
    }

    /// Puts the key on split questions: the sums, the annulled questions and the annulled propositions.
    pub fn apply(&self, questions: &mut [Question]) -> usize {
        let applied = crate::exam::apply_answer_key(questions, &self.answer_key());
        for q in questions.iter_mut() {
            q.annulled = self.annulled.contains(&q.number);
            q.void = self.void_options.iter().filter(|v| v.0 == q.number).filter_map(|v| v.1.parse::<u32>().ok()).fold(0, |a, b| a | b);
        }
        applied
    }

    /// The exam's questions without the other language's block.
    pub fn keep_language(&self, questions: Vec<Question>) -> Vec<Question> {
        let Some(keep) = self.language else { return questions };
        let mut block = 0;
        questions
            .into_iter()
            .filter(|q| {
                block += usize::from(q.number == 56);
                q.number < 56 || block == keep + 1
            })
            .collect()
    }
}

#[rustfmt::skip]
// uepg-2025-grupo-1
const G1_ANSWERS: &[(u32, &str)] = &[
    (1, "24"), (2, "05"), (3, "31"), (4, "17"), (5, "20"), (6, "22"), (7, "13"), (8, "28"),
    (9, "30"), (10, "23"), (11, "03"), (12, "14"), (13, "18"), (14, "11"), (15, "21"), (16, "12"),
    (17, "19"), (18, "30"), (19, "15"), (20, "09"), (21, "31"), (22, "27"), (23, "07"), (24, "04"),
    (25, "28"), (26, "29"), (27, "10"), (28, "31"), (29, "25"), (30, "20"), (31, "26"), (32, "22"),
    (33, "10"), (34, "30"), (35, "05"), (36, "29"), (37, "17"), (38, "12"), (39, "13"), (40, "31"),
    (41, "06"), (42, "25"), (43, "21"), (44, "27"), (45, "24"),
];
const G1_ANNULLED: &[u32] = &[];
const G1_VOID: &[(u32, &str)] = &[];

// uepg-2025-grupo-2
const G2_ANSWERS: &[(u32, &str)] = &[
    (1, "24"), (2, "05"), (3, "31"), (4, "17"), (5, "20"), (6, "22"), (7, "13"), (8, "28"),
    (9, "30"), (10, "23"), (11, "03"), (12, "14"), (13, "18"), (14, "11"), (15, "21"), (16, "12"),
    (17, "19"), (18, "30"), (19, "15"), (20, "09"), (21, "31"), (22, "27"), (23, "07"), (24, "04"),
    (25, "28"), (26, "29"), (27, "10"), (28, "31"), (29, "25"), (30, "20"), (31, "24"), (32, "30"),
    (34, "15"), (35, "11"), (36, "03"), (37, "20"), (38, "30"), (39, "27"), (40, "12"), (41, "09"),
    (42, "13"), (43, "31"), (44, "10"), (45, "28"),
];
const G2_ANNULLED: &[u32] = &[33];
const G2_VOID: &[(u32, &str)] = &[(38, "01"), (40, "01")];

// uepg-2025-grupo-3
const G3_ANSWERS: &[(u32, &str)] = &[
    (1, "24"), (2, "05"), (3, "31"), (4, "17"), (5, "20"), (6, "22"), (7, "13"), (8, "28"),
    (9, "30"), (10, "23"), (11, "03"), (12, "14"), (13, "18"), (14, "11"), (15, "21"), (16, "09"),
    (17, "07"), (18, "20"), (19, "23"), (20, "10"), (21, "31"), (22, "19"), (23, "06"), (24, "12"),
    (25, "28"), (26, "15"), (27, "18"), (28, "25"), (29, "27"), (30, "17"), (31, "26"), (32, "22"),
    (33, "10"), (34, "30"), (35, "05"), (36, "29"), (37, "17"), (38, "12"), (39, "13"), (40, "31"),
    (41, "06"), (42, "25"), (43, "21"), (44, "27"), (45, "24"),
];
const G3_ANNULLED: &[u32] = &[];
const G3_VOID: &[(u32, &str)] = &[];

// uepg-2025-grupo-4
const G4_ANSWERS: &[(u32, &str)] = &[
    (1, "18"), (2, "23"), (3, "06"), (4, "31"), (5, "27"), (6, "14"), (7, "29"), (8, "03"),
    (9, "12"), (10, "05"), (11, "28"), (12, "31"), (13, "25"), (14, "30"), (15, "28"), (16, "10"),
    (17, "29"), (18, "05"), (19, "07"), (20, "15"), (21, "11"), (22, "30"), (23, "17"), (24, "12"),
    (25, "22"), (26, "31"), (27, "26"), (28, "23"), (29, "21"), (30, "27"), (31, "24"), (32, "30"),
    (34, "15"), (35, "11"), (36, "03"), (37, "20"), (38, "30"), (39, "27"), (40, "12"), (41, "09"),
    (42, "13"), (43, "31"), (44, "10"), (45, "28"),
];
const G4_ANNULLED: &[u32] = &[33];
const G4_VOID: &[(u32, &str)] = &[(38, "01"), (40, "01")];

// uepg-2025-grupo-5
const G5_ANSWERS: &[(u32, &str)] = &[
    (1, "18"), (2, "23"), (3, "06"), (4, "31"), (5, "27"), (6, "14"), (7, "29"), (8, "03"),
    (9, "12"), (10, "05"), (11, "28"), (12, "31"), (13, "25"), (14, "30"), (15, "28"), (16, "12"),
    (17, "19"), (18, "30"), (19, "15"), (20, "09"), (21, "31"), (22, "27"), (23, "07"), (24, "04"),
    (25, "28"), (26, "29"), (27, "10"), (28, "31"), (29, "25"), (30, "20"), (31, "24"), (32, "30"),
    (34, "15"), (35, "11"), (36, "03"), (37, "20"), (38, "30"), (39, "27"), (40, "12"), (41, "09"),
    (42, "13"), (43, "31"), (44, "10"), (45, "28"),
];
const G5_ANNULLED: &[u32] = &[33];
const G5_VOID: &[(u32, &str)] = &[(38, "01"), (40, "01")];

// uepg-2025-grupo-6
const G6_ANSWERS: &[(u32, &str)] = &[
    (1, "24"), (2, "05"), (3, "31"), (4, "17"), (5, "20"), (6, "22"), (7, "13"), (8, "28"),
    (9, "30"), (10, "23"), (11, "03"), (12, "14"), (13, "18"), (14, "11"), (15, "21"), (16, "10"),
    (17, "29"), (18, "05"), (19, "07"), (20, "15"), (21, "11"), (22, "30"), (23, "17"), (24, "12"),
    (25, "22"), (26, "31"), (27, "26"), (28, "23"), (29, "21"), (30, "27"), (31, "26"), (32, "22"),
    (33, "10"), (34, "30"), (35, "05"), (36, "29"), (37, "17"), (38, "12"), (39, "13"), (40, "31"),
    (41, "06"), (42, "25"), (43, "21"), (44, "27"), (45, "24"),
];
const G6_ANNULLED: &[u32] = &[];
const G6_VOID: &[(u32, &str)] = &[];

// uepg-2025-cg-ingles
const CG_EN_ANSWERS: &[(u32, &str)] = &[
    (1, "12"), (2, "11"), (3, "06"), (4, "13"), (5, "07"), (6, "05"), (7, "14"), (8, "13"),
    (9, "15"), (10, "03"), (11, "14"), (12, "09"), (13, "07"), (14, "15"), (15, "03"), (16, "10"),
    (17, "11"), (18, "07"), (19, "12"), (20, "05"), (21, "03"), (22, "14"), (23, "09"), (24, "06"),
    (25, "10"), (26, "13"), (27, "07"), (28, "15"), (29, "11"), (30, "06"), (31, "14"), (32, "10"),
    (33, "13"), (34, "15"), (35, "07"), (36, "12"), (37, "15"), (38, "13"), (39, "07"), (40, "05"),
    (41, "05"), (42, "11"), (43, "15"), (44, "14"), (45, "03"), (46, "15"), (47, "03"), (48, "09"),
    (49, "14"), (50, "13"), (51, "06"), (52, "07"), (53, "12"), (54, "15"), (55, "11"), (56, "09"),
    (57, "11"), (58, "15"), (59, "03"), (60, "14"),
];
const CG_EN_ANNULLED: &[u32] = &[];
const CG_EN_VOID: &[(u32, &str)] = &[];

// uepg-2025-cg-espanhol
const CG_ES_ANSWERS: &[(u32, &str)] = &[
    (1, "12"), (2, "11"), (3, "06"), (4, "13"), (5, "07"), (6, "05"), (7, "14"), (8, "13"),
    (9, "15"), (10, "03"), (11, "14"), (12, "09"), (13, "07"), (14, "15"), (15, "03"), (16, "10"),
    (17, "11"), (18, "07"), (19, "12"), (20, "05"), (21, "03"), (22, "14"), (23, "09"), (24, "06"),
    (25, "10"), (26, "13"), (27, "07"), (28, "15"), (29, "11"), (30, "06"), (31, "14"), (32, "10"),
    (33, "13"), (34, "15"), (35, "07"), (36, "12"), (37, "15"), (38, "13"), (39, "07"), (40, "05"),
    (41, "05"), (42, "11"), (43, "15"), (44, "14"), (45, "03"), (46, "15"), (47, "03"), (48, "09"),
    (49, "14"), (50, "13"), (51, "06"), (52, "07"), (53, "12"), (54, "15"), (55, "11"), (56, "10"),
    (57, "14"), (58, "07"), (59, "11"), (60, "06"),
];
const CG_ES_ANNULLED: &[u32] = &[];
const CG_ES_VOID: &[(u32, &str)] = &[];

pub const ENTRIES: [Entry; 8] = [
    Entry {
        id: "uepg-2025-grupo-1",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Grupo 1 (Português, Matemática, História)",
        style: "uepg",
        download: Download { file: "uepg-2025-grupo-1-por-mat-his.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Grupo-1-Por-Mat-His-2025.pdf", sha256: "32d080b00461dd5c70c091341fcf4f3cd655f4a1bf550d0d9a5d6df822c34d63", size: 669003 },
        questions: 45,
        language: None,
        answers: G1_ANSWERS,
        annulled: G1_ANNULLED,
        void_options: G1_VOID,
    },
    Entry {
        id: "uepg-2025-grupo-2",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Grupo 2 (Português, Matemática, Física)",
        style: "uepg",
        download: Download { file: "uepg-2025-grupo-2-por-mat-fis.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Grupo-2-Por-Mat-Fis-2025.pdf", sha256: "529624c579bb2982a024a5bb346af1cb1cdb0a0db73b230dc6e6f712fbc66c4e", size: 802620 },
        questions: 45,
        language: None,
        answers: G2_ANSWERS,
        annulled: G2_ANNULLED,
        void_options: G2_VOID,
    },
    Entry {
        id: "uepg-2025-grupo-3",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Grupo 3 (Português, Geografia, História)",
        style: "uepg",
        download: Download { file: "uepg-2025-grupo-3-por-geo-his.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Grupo-3-Por-Geo-His-2025.pdf", sha256: "93098a2684a0a6440094b44fc59fb984b0abe8c3425375c5da52bd9656f1702d", size: 397524 },
        questions: 45,
        language: None,
        answers: G3_ANSWERS,
        annulled: G3_ANNULLED,
        void_options: G3_VOID,
    },
    Entry {
        id: "uepg-2025-grupo-4",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Grupo 4 (Química, Biologia, Física)",
        style: "uepg",
        download: Download { file: "uepg-2025-grupo-4-qui-bio-fis.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Grupo-4-Qui-Bio-Fis-2025.pdf", sha256: "b979dd3dba13e6d51c47c8aaaf0ad22c4033e3f35797ab453d71c4268527f35f", size: 873637 },
        questions: 45,
        language: None,
        answers: G4_ANSWERS,
        annulled: G4_ANNULLED,
        void_options: G4_VOID,
    },
    Entry {
        id: "uepg-2025-grupo-5",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Grupo 5 (Química, Matemática, Física)",
        style: "uepg",
        download: Download { file: "uepg-2025-grupo-5-qui-mat-fis.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Grupo-5-Qui-Mat-Fis-2025.pdf", sha256: "fd2c2d78fef251f0f5db64c6d289851af59d6101a1f1f7d8069c2715056c3e93", size: 824170 },
        questions: 45,
        language: None,
        answers: G5_ANSWERS,
        annulled: G5_ANNULLED,
        void_options: G5_VOID,
    },
    Entry {
        id: "uepg-2025-grupo-6",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Grupo 6 (Português, Biologia, História)",
        style: "uepg",
        download: Download { file: "uepg-2025-grupo-6-por-biol-his.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Grupo-6-Por-Biol-His-2025.pdf", sha256: "90bdd09de2b96e0fd20779cc10eb6a8aa9043abc984e8308540771d36589c519", size: 593111 },
        questions: 45,
        language: None,
        answers: G6_ANSWERS,
        annulled: G6_ANNULLED,
        void_options: G6_VOID,
    },
    Entry {
        id: "uepg-2025-cg-ingles",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Conhecimentos Gerais (com Inglês)",
        style: "uepg",
        download: Download { file: "uepg-2025-conhecimentos-gerais.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Conhecimentos-Gerais-2025.pdf", sha256: "b359eea047702770a9793b52a694e958c3b17d9314081178f44f5fe4cba3239c", size: 971076 },
        questions: 60,
        language: Some(0),
        answers: CG_EN_ANSWERS,
        annulled: CG_EN_ANNULLED,
        void_options: CG_EN_VOID,
    },
    Entry {
        id: "uepg-2025-cg-espanhol",
        family: "uepg",
        year: 2025,
        title: "Vestibular 2025 – Conhecimentos Gerais (com Espanhol)",
        style: "uepg",
        download: Download { file: "uepg-2025-conhecimentos-gerais.pdf", url: "https://www2.uepg.br/cps/wp-content/uploads/sites/270/2025/12/Conhecimentos-Gerais-2025.pdf", sha256: "b359eea047702770a9793b52a694e958c3b17d9314081178f44f5fe4cba3239c", size: 971076 },
        questions: 60,
        language: Some(1),
        answers: CG_ES_ANSWERS,
        annulled: CG_ES_ANNULLED,
        void_options: CG_ES_VOID,
    },
];

/// The biggest catalogue file Jay-S downloads (the catalogue test keeps every entry under it too).
const MAX_DOWNLOAD: u64 = 20 << 20;

/// One at a time: a second call for an entry waits, then finds it ready. (One lock for all entries,
/// not one per entry: the app runs a single long job anyway, and this is stricter.)
static GETTING: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum CatalogState {
    NotDownloaded,
    Ready { exam: String },
}

/// What the page shows of an entry: never the URL or the key.
#[derive(Debug, Clone, Serialize)]
pub struct CatalogItem {
    pub id: &'static str,
    pub family: &'static str,
    pub year: u32,
    pub title: &'static str,
    pub style: &'static str,
    pub questions: u32,
    /// Bytes to download.
    pub size: u64,
    pub official_key: bool,
    #[serde(flatten)]
    pub state: CatalogState,
}

/// `<data>/catalog/<entry id>.json`: where a downloaded entry went.
#[derive(Serialize, Deserialize)]
struct Record {
    material: String,
    exam: String,
}

impl Store {
    fn catalog_dir(&self) -> std::path::PathBuf {
        self.root().join("catalog")
    }

    /// The exam built from `entry`, if it still exists (the student may have deleted it).
    fn catalog_exam(&self, entry: &Entry) -> Option<String> {
        let record: Record = read_json(&self.catalog_dir().join(format!("{}.json", entry.id))).ok()?;
        self.exam(&record.exam).ok().map(|e| e.id)
    }

    pub fn catalog(&self) -> Vec<CatalogItem> {
        ENTRIES
            .iter()
            .map(|e| CatalogItem {
                id: e.id,
                family: e.family,
                year: e.year,
                title: e.title,
                style: e.style,
                questions: e.questions,
                size: e.download.size,
                official_key: true,
                state: self.catalog_exam(e).map_or(CatalogState::NotDownloaded, |exam| CatalogState::Ready { exam }),
            })
            .collect()
    }

    /// Downloads a catalogue exam (checked against its pinned size and SHA-256), reads it, puts the
    /// official key on it and saves it as an exam; no Jay-AI. Only an id from `ENTRIES` is taken.
    /// Returns the exam's id; an entry already made returns its exam without downloading.
    pub fn get_ready_made(&self, entry_id: &str, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<String, BuildError> {
        let entry = ENTRIES.iter().find(|e| e.id == entry_id).ok_or(StoreError::NotFound)?;
        // Belt and braces over the catalogue test: one host, a size cap.
        let d = &entry.download;
        if !d.url.starts_with("https://www2.uepg.br/") || d.size > MAX_DOWNLOAD {
            return Err(AiError::Network("not an allowed download".into()).into());
        }
        let _one = GETTING.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(exam) = self.catalog_exam(entry) {
            return Ok(exam);
        }
        std::fs::create_dir_all(self.catalog_dir()).map_err(StoreError::from)?;
        let pdf = self.catalog_dir().join(format!("{}.pdf", entry.id));
        // shortcut: ureq ignores the Windows system proxy, so behind one the download fails with a
        // network error; upgrade when a student reports it (same for the model download).
        download(d, &pdf, progress, cancel)?;
        let bytes = std::fs::read(&pdf).map_err(StoreError::from);
        // The store keeps its own copy (`original.pdf`); the downloaded file is not needed.
        let _ = std::fs::remove_file(&pdf);
        let bytes = bytes?;
        let format = extract::detect(&bytes[..bytes.len().min(4096)]).ok_or(StoreError::Extract(extract::ExtractError::Unsupported))?;
        // The same file may already be a material (the student's own, or the other Conhecimentos
        // Gerais entry's): that one is not ours to remove.
        let had = self.materials().iter().any(|m| m.sha256.as_deref() == Some(d.sha256));
        let name: String = format!("{}.pdf", entry.title).chars().take(200).collect();
        let material = self.import_bytes(&name, format, bytes)?;
        let built = self.build_catalog_exam(entry, &material.id);
        if built.is_err() && !had {
            let _ = self.delete(&material.id);
        }
        built
    }

    fn build_catalog_exam(&self, entry: &Entry, material: &str) -> Result<String, BuildError> {
        let all = exam::split_questions(&self.text(material, usize::MAX)?);
        let mut questions = entry.keep_language(all);
        if questions.iter().map(|q| q.number).ne(1..=entry.questions) {
            return Err(BuildError::Empty);
        }
        for q in &mut questions {
            q.source = Some(Source { material: material.to_owned(), number: Some(q.number) });
        }
        if entry.apply(&mut questions) != entry.answers.len() {
            return Err(BuildError::Empty);
        }
        let exam = Exam { id: id::new_id(), title: entry.title.to_owned(), style: entry.style.to_owned(), created: now_ms(), questions, materials: vec![material.to_owned()] };
        self.save_exam(&exam)?;
        let record = write_json(&self.catalog_dir().join(format!("{}.json", entry.id)), &Record { material: material.to_owned(), exam: exam.id.clone() });
        if let Err(e) = record {
            let _ = self.delete_exam(&exam.id);
            return Err(StoreError::from(e).into());
        }
        Ok(exam.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn hex64(s: &str) -> bool {
        s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }

    fn entry(id: &str) -> &'static Entry {
        ENTRIES.iter().find(|e| e.id == id).unwrap()
    }

    #[test]
    fn the_catalogue_is_well_formed() {
        let mut ids: Vec<_> = ENTRIES.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ENTRIES.len(), "ids repeat");
        assert!(hex64(KEY_SHA256) && KEY_URL.starts_with("https://www2.uepg.br/"));
        for e in &ENTRIES {
            let d = &e.download;
            assert!(d.url.starts_with("https://www2.uepg.br/") && d.url.ends_with(".pdf"), "{}: {}", e.id, d.url);
            assert!(hex64(d.sha256), "{}", e.id);
            assert!((1..=20 << 20).contains(&d.size), "{}", e.id);
            assert!(d.file.ends_with(".pdf") && !d.file.contains(['/', '\\']), "{}", e.id);
            assert!(exam::style(e.style).is_some() && e.family == "uepg" && e.year == 2025, "{}", e.id);
            // Every question has a sum or is annulled, never both. Conhecimentos Gerais has four
            // propositions (01-08); the groups five (01-16).
            let n = if e.language.is_some() { 4 } else { 5 };
            let mut numbers: Vec<u32> = e.answers.iter().map(|a| a.0).chain(e.annulled.iter().copied()).collect();
            numbers.sort();
            assert_eq!(numbers, (1..=e.questions).collect::<Vec<_>>(), "{}", e.id);
            for (q, sum) in e.answers {
                let s: u32 = sum.parse().unwrap();
                assert!(sum.len() == 2 && (1..1 << n).contains(&s), "{} question {q}: {sum}", e.id);
                for (vq, v) in e.void_options.iter().filter(|v| v.0 == *q) {
                    let v: u32 = v.parse().unwrap();
                    assert!(v.is_power_of_two() && v < 1 << n && s & v == 0, "{} question {vq}: proposition {v} annulled but counted", e.id);
                }
            }
            assert!(e.void_options.iter().all(|(q, _)| e.answers.iter().any(|a| a.0 == *q)), "{}: a void proposition of a question without a sum", e.id);
        }
        // What the edital's parecer says (Física 33 annulled; 38 and 40 lose proposition 01).
        let g2 = entry("uepg-2025-grupo-2");
        assert_eq!(g2.annulled, [33]);
        assert_eq!(g2.void_options, [(38, "01"), (40, "01")]);
        assert!(g2.answers.contains(&(38, "30")) && g2.answers.contains(&(40, "12")) && g2.answers.contains(&(24, "04")));
        assert!(entry("uepg-2025-grupo-4").answers.contains(&(11, "28")));
    }

    #[test]
    fn apply_marks_annulled_questions_and_propositions() {
        let q = |number| Question { number, kind: exam::QuestionKind::Sum, passage: None, stem: String::new(), options: (0..5).map(|i| exam::QOption { label: format!("{:02}", 1 << i), text: String::new() }).collect(), answer: None, answer_by_ai: false, topic: None, explanation: None, source: None, annulled: false, void: 0 };
        let mut qs = [33, 38, 24].map(q).to_vec();
        assert_eq!(entry("uepg-2025-grupo-2").apply(&mut qs), 2);
        assert_eq!(qs.iter().map(|q| (q.annulled, q.void, q.answer.as_deref())).collect::<Vec<_>>(), [(true, 0, None), (false, 1, Some("30")), (false, 0, Some("04"))]);
    }

    #[test]
    fn keep_language_drops_the_other_block() {
        let q = |number| Question { number, kind: exam::QuestionKind::Sum, passage: None, stem: String::new(), options: vec![], answer: None, answer_by_ai: false, topic: None, explanation: None, source: None, annulled: false, void: 0 };
        let all = || [55, 56, 57, 56, 57].map(q).to_vec();
        let kept = |e: &str| entry(e).keep_language(all()).iter().map(|q| q.number).collect::<Vec<_>>();
        assert_eq!(kept("uepg-2025-cg-ingles"), [55, 56, 57]);
        assert_eq!(kept("uepg-2025-cg-espanhol"), [55, 56, 57]);
        assert_eq!(kept("uepg-2025-grupo-1").len(), 5);
        // The two blocks are told apart: the second 56 is Spanish's.
        let tag = |mut qs: Vec<Question>| {
            qs[3].stem = "es".into();
            entry("uepg-2025-cg-espanhol").keep_language(qs).iter().map(|q| q.stem.clone()).collect::<Vec<_>>()
        };
        assert_eq!(tag(all()), ["", "es", ""]);
    }

    fn temp_store() -> (Store, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("jay-study-catalog-{}", id::new_id()));
        (Store::open(&root).unwrap(), root)
    }

    /// Every file and folder under `dir`, sorted.
    fn tree(dir: &std::path::Path) -> Vec<String> {
        let mut out = vec![];
        let mut todo = vec![dir.to_path_buf()];
        while let Some(d) = todo.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                out.push(e.path().strip_prefix(dir).unwrap().to_string_lossy().into_owned());
                if e.path().is_dir() {
                    todo.push(e.path());
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn hostile_ids_touch_nothing() {
        let (store, root) = temp_store();
        let before = tree(&root);
        let cancel = AtomicBool::new(false);
        let long = "a".repeat(10_000);
        for bad in ["../x", "", "https://evil", "https://www2.uepg.br/cps/x.pdf", "..\\..", "uepg-2025-grupo-1/../x", "uepg-2025-grupo-1x", "uepg-2025-grupo-1 ", "UEPG-2025-GRUPO-1", long.as_str()] {
            let r = store.get_ready_made(bad, &mut |_, _| panic!("downloading {bad:?}"), &cancel);
            assert!(matches!(r, Err(BuildError::Store(StoreError::NotFound))), "{bad:?}");
        }
        assert_eq!(tree(&root), before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_tampered_download_leaves_nothing() {
        let (store, root) = temp_store();
        let e = entry("uepg-2025-grupo-3");
        // A `.part` of the right size and the wrong bytes: checked without the network.
        let part = root.join("catalog").join(format!("{}.part", e.id));
        std::fs::create_dir_all(part.parent().unwrap()).unwrap();
        std::fs::write(&part, vec![b'x'; e.download.size as usize]).unwrap();
        let r = store.get_ready_made(e.id, &mut |_, _| {}, &AtomicBool::new(false));
        assert!(matches!(r, Err(BuildError::Ai(AiError::Mismatch))), "{r:?}");
        assert!(store.materials().is_empty() && store.exams().is_empty());
        assert_eq!(tree(&root), ["catalog", "materials"]);
        assert!(store.catalog().iter().all(|i| i.state == CatalogState::NotDownloaded));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_catalogue_lists_every_entry_and_forgets_a_deleted_exam() {
        let (store, root) = temp_store();
        let items = store.catalog();
        assert_eq!(items.len(), ENTRIES.len());
        assert!(items.iter().all(|i| i.official_key && i.state == CatalogState::NotDownloaded));
        // A record pointing at an exam that is gone is not ready.
        std::fs::create_dir_all(root.join("catalog")).unwrap();
        write_json(&root.join("catalog/uepg-2025-grupo-1.json"), &Record { material: id::new_id(), exam: id::new_id() }).unwrap();
        assert_eq!(store.catalog()[0].state, CatalogState::NotDownloaded);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Needs the local dataset: `JAYS_DATASET=<folder with exam_sum/> cargo test catalog -- --ignored`.
    /// The real PDFs stand in for the download: a complete `.part` is verified without the network.
    #[test]
    #[ignore]
    fn the_catalogue_builds_every_real_exam() {
        let dir = std::env::var("JAYS_DATASET").expect("JAYS_DATASET");
        let (store, root) = temp_store();
        let cancel = AtomicBool::new(false);
        for e in &ENTRIES {
            let part = root.join("catalog").join(format!("{}.part", e.id));
            std::fs::create_dir_all(part.parent().unwrap()).unwrap();
            std::fs::copy(format!("{dir}/exam_sum/{}", e.download.file), &part).unwrap();
            let exam_id = store.get_ready_made(e.id, &mut |_, _| {}, &cancel).unwrap();
            let exam = store.exam(&exam_id).unwrap();
            assert_eq!((exam.questions.len(), exam.style.as_str()), (e.questions as usize, "uepg"), "{}", e.id);
            assert!(exam.questions.iter().all(|q| q.annulled == e.annulled.contains(&q.number) && (q.annulled || q.answer.is_some()) && q.source.is_some()), "{}", e.id);
            let void = exam.questions.iter().filter(|q| q.void != 0).count();
            assert_eq!(void, e.void_options.iter().map(|v| v.0).collect::<std::collections::BTreeSet<_>>().len(), "{}", e.id);
            // The second call: the same exam, nothing downloaded or imported again.
            let materials = store.materials().len();
            assert_eq!(store.get_ready_made(e.id, &mut |_, _| panic!("downloaded again"), &cancel).unwrap(), exam_id);
            assert_eq!(store.materials().len(), materials);
            assert!(!root.join("catalog").join(format!("{}.pdf", e.id)).exists() && !part.exists());
            assert_eq!(store.catalog().iter().find(|i| i.id == e.id).unwrap().state, CatalogState::Ready { exam: exam_id });
            println!("{}: {} questions, {} annulled, {} with a void proposition, answers on {}", e.id, exam.questions.len(), e.annulled.len(), void, exam.questions.iter().filter(|q| q.answer.is_some()).count());
        }
        // Eight exams from seven materials (the two Conhecimentos Gerais entries share one file).
        assert_eq!((store.exams().len(), store.materials().len()), (8, 7));
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Needs the local dataset: `JAYS_DATASET=<folder with exam_sum/> cargo test catalog -- --ignored`.
    #[test]
    #[ignore]
    fn the_catalogue_matches_the_real_exams() {
        let dir = std::env::var("JAYS_DATASET").expect("JAYS_DATASET");
        for e in &ENTRIES {
            let bytes = std::fs::read(format!("{dir}/exam_sum/{}", e.download.file)).expect("read");
            assert_eq!(bytes.len() as u64, e.download.size, "{}: size", e.id);
            assert_eq!(id::hex(&Sha256::digest(&bytes)), e.download.sha256, "{}: sha256", e.id);
            let format = extract::detect(&bytes[..bytes.len().min(4096)]).unwrap();
            let all = exam::split_questions(&extract::extract(format, bytes).unwrap().text);
            let mut questions = e.keep_language(all);
            let n = if e.language.is_some() { 4 } else { 5 };
            assert_eq!(questions.iter().map(|q| q.number).collect::<Vec<_>>(), (1..=e.questions).collect::<Vec<_>>(), "{}", e.id);
            assert!(questions.iter().all(|q| q.kind == exam::QuestionKind::Sum && q.options.len() == n), "{}", e.id);
            let applied = exam::apply_answer_key(&mut questions, &e.answer_key());
            assert_eq!(applied, e.answers.len(), "{}", e.id);
            for q in &questions {
                let sum = e.answers.iter().find(|a| a.0 == q.number).map(|a| a.1.to_string());
                assert_eq!(q.answer, sum, "{} question {}", e.id, q.number);
            }
            println!("{}: {} questions 1-{}, {applied} answers applied, annulled {:?}, void {:?}", e.id, questions.len(), e.questions, e.annulled, e.void_options);
        }
    }

    /// Needs the local dataset: `JAYS_DATASET=<dataset> cargo test the_real_edital -- --ignored --nocapture`.
    /// The student attaches the real edital next to each exam: the 7 real UEPG 2025 exams get exactly
    /// the catalogue's answers (annulled questions and propositions too), and no other exam in the
    /// dataset (other years, ENEM, UEM…) gets a single answer from it.
    #[test]
    #[ignore]
    fn the_real_edital_answers_its_own_exams_and_no_other() {
        use crate::classify::MaterialKind;
        let dir = std::env::var("JAYS_DATASET").expect("JAYS_DATASET");
        let (store, root) = temp_store();
        let key = store.import(std::path::Path::new(&format!("{dir}/key/uepg-2025-vestibular-key.pdf"))).unwrap();
        assert_eq!(key.kind, MaterialKind::Key);
        let landed = |file: &str| {
            let exam = store.import(std::path::Path::new(file)).unwrap();
            let qs = store.material_questions(&exam.id, Some(&key.id)).unwrap();
            let best = store.best_key(&exam.id);
            store.delete(&exam.id).unwrap();
            (qs, best)
        };
        let (mut answered, mut annulled, mut void, mut wrong) = (0, 0, 0, 0);
        let mut mine = vec![];
        for e in &ENTRIES {
            // Conhecimentos Gerais is one file for two entries: its 56-60 come twice (Inglês, Espanhol).
            if e.language == Some(1) {
                continue;
            }
            let file = format!("{dir}/exam_sum/{}", e.download.file);
            mine.push(file.clone());
            let (qs, best) = landed(&file);
            println!("{}: {} questions, {} answered", e.id, qs.len(), qs.iter().filter(|q| q.answer.is_some()).count());
            assert_eq!(best.as_deref(), Some(key.id.as_str()), "{}: best_key", e.id);
            // What the catalogue says for the same questions.
            let text = |f: &str| {
                let bytes = std::fs::read(f).unwrap();
                exam::split_questions(&extract::extract(extract::detect(&bytes[..bytes.len().min(4096)]).unwrap(), bytes).unwrap().text)
            };
            let mut want = text(&file);
            if e.language.is_some() {
                let (mut english, mut spanish) = (e.keep_language(want.clone()), entry("uepg-2025-cg-espanhol").keep_language(want));
                e.apply(&mut english);
                entry("uepg-2025-cg-espanhol").apply(&mut spanish);
                // 1-55 and the first 56-60 from Inglês, the second 56-60 from Espanhol.
                want = english.into_iter().chain(spanish.into_iter().filter(|q| q.number >= 56)).collect();
            } else {
                e.apply(&mut want);
            }
            assert_eq!(qs.len(), want.len(), "{}", e.id);
            for (got, want) in qs.iter().zip(&want) {
                assert_eq!(got.number, want.number, "{}", e.id);
                wrong += usize::from(got.answer.is_some() && got.answer != want.answer);
                assert!(got.answer == want.answer && got.annulled == want.annulled && got.void == want.void, "{} question {}: {:?} vs {:?}", e.id, got.number, (&got.answer, got.annulled, got.void), (&want.answer, want.annulled, want.void));
                answered += usize::from(got.answer.is_some());
                annulled += usize::from(got.annulled);
                void += usize::from(got.void != 0);
            }
        }
        println!("the 7 exams: {answered} answers, {annulled} annulled questions, {void} questions with a void proposition, {wrong} wrong");
        assert_eq!(wrong, 0);
        // Every other exam in the dataset: nothing lands, and the edital is nobody's automatic key.
        let others: Vec<_> = ["exam_sum", "exam", "both", "content"]
            .iter()
            .flat_map(|d| std::fs::read_dir(format!("{dir}/{d}")).unwrap().map(|f| f.unwrap().path()))
            .filter(|p| p.extension().is_some_and(|x| x == "pdf") && !mine.contains(&p.to_string_lossy().replace('\\', "/")))
            .collect();
        let started = std::time::Instant::now();
        let mut checked = 0;
        for p in &others {
            let (qs, best) = landed(&p.to_string_lossy());
            assert!(qs.iter().all(|q| q.answer.is_none() && !q.annulled && q.void == 0) && best.is_none(), "{}", p.display());
            checked += 1;
        }
        println!("{checked} other files: no answer, no annulment, no automatic key ({:.0} s)", started.elapsed().as_secs_f32());
        std::fs::remove_dir_all(root).unwrap();
    }
}
