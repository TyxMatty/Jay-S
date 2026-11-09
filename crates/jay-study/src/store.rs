//! The student's data folder (docs/ARQUITETURA.md, section 6). Every write is atomic (`.tmp` +
//! rename), every path is built here from a checked id, and the attached file is a copy: the
//! student's original is never touched.
//!
//! ```text
//! <root>/materials/<id>/meta.json   what the list shows (small: listing reads only these)
//!                     /original.<ext>
//!                     /text.txt     the extracted text, pages split by \f
//! ```

use crate::classify::{Classification, MaterialKind, classify};
use crate::extract::{self, ExtractError, Format, Warning};
use crate::id;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The biggest JSON file read back (a corrupted or planted huge file stops here).
pub const MAX_JSON_BYTES: u64 = 64 << 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    /// The attached file's name, for showing only (never a path).
    pub name: String,
    pub format: Format,
    pub size: u64,
    /// When it was attached, in ms since 1970.
    pub added: u64,
    pub pages: u32,
    /// Letters and digits in the text (how much there is to study).
    pub chars: u64,
    pub ocr_pages: u32,
    pub kind: MaterialKind,
    /// What the rules said; `kind` differs when the student changed it.
    pub detected: Classification,
    pub warnings: Vec<Warning>,
}

#[derive(Debug)]
pub enum StoreError {
    /// The id isn't one (or the item doesn't exist).
    NotFound,
    Extract(ExtractError),
    Io(std::io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::Extract(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Writes `bytes` to `path` through a `.tmp` beside it and a rename: a crash leaves the old file or the
/// new one, never half of one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    write_atomic(path, &serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?)
}

/// Reads a JSON file of at most `MAX_JSON_BYTES`.
pub fn read_json<T: DeserializeOwned>(path: &Path) -> std::io::Result<T> {
    let mut data = Vec::new();
    std::fs::File::open(path)?.take(MAX_JSON_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_JSON_BYTES {
        return Err(std::io::Error::other("file too big"));
    }
    serde_json::from_slice(&data).map_err(std::io::Error::other)
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    /// The store at `root` (the app's data folder), creating its folders.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(root.join("materials"))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The folder of material `id`, only for a well-formed id (docs/ARQUITETURA.md, section 7, item 2).
    fn material_dir(&self, material: &str) -> Result<PathBuf, StoreError> {
        if !id::is_valid(material) {
            return Err(StoreError::NotFound);
        }
        Ok(self.root.join("materials").join(material))
    }

    /// Every material, newest first. Folders that aren't a material (a half-made import, a stray
    /// folder) are skipped.
    pub fn materials(&self) -> Vec<Material> {
        let Ok(entries) = std::fs::read_dir(self.root.join("materials")) else { return vec![] };
        let mut list: Vec<Material> = entries
            .flatten()
            .filter(|e| e.file_name().to_str().is_some_and(id::is_valid))
            .filter_map(|e| read_json::<Material>(&e.path().join("meta.json")).ok())
            .filter(|m| id::is_valid(&m.id))
            .collect();
        list.sort_by(|a, b| b.added.cmp(&a.added));
        list
    }

    pub fn material(&self, material: &str) -> Result<Material, StoreError> {
        read_json(&self.material_dir(material)?.join("meta.json")).map_err(|_| StoreError::NotFound)
    }

    /// Copies `path` in, reads its text and tells what it is. `name` is what the list shows.
    pub fn import(&self, path: &Path) -> Result<Material, StoreError> {
        let meta = std::fs::metadata(path)?;
        if !meta.is_file() {
            return Err(StoreError::Extract(ExtractError::Unsupported));
        }
        if meta.len() > extract::MAX_FILE_BYTES {
            return Err(StoreError::Extract(ExtractError::TooBig));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.take(extract::MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
        let format = extract::detect(&bytes[..bytes.len().min(4096)]).ok_or(StoreError::Extract(ExtractError::Unsupported))?;
        let name: String = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().chars().take(200).collect();
        self.import_bytes(&name, format, bytes)
    }

    pub fn import_bytes(&self, name: &str, format: Format, bytes: Vec<u8>) -> Result<Material, StoreError> {
        let new = id::new_id();
        let dir = self.material_dir(&new)?;
        std::fs::create_dir_all(&dir)?;
        let result = (|| {
            write_atomic(&dir.join(format!("original.{}", format.extension())), &bytes)?;
            let size = bytes.len() as u64;
            let extracted = extract::extract(format, bytes).map_err(StoreError::Extract)?;
            let detected = classify(&extracted.text);
            write_atomic(&dir.join("text.txt"), extracted.text.as_bytes())?;
            let material = Material {
                id: new.clone(),
                name: name.to_owned(),
                format,
                size,
                added: now_ms(),
                pages: extracted.pages,
                chars: extracted.text.chars().filter(|c| c.is_alphanumeric()).count() as u64,
                ocr_pages: extracted.ocr_pages,
                kind: detected.kind,
                detected,
                warnings: extracted.warnings,
            };
            // meta.json last: until it exists, the folder isn't listed.
            write_json(&dir.join("meta.json"), &material)?;
            Ok(material)
        })();
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        result
    }

    pub fn set_kind(&self, material: &str, kind: MaterialKind) -> Result<Material, StoreError> {
        let mut m = self.material(material)?;
        m.kind = kind;
        write_json(&self.material_dir(material)?.join("meta.json"), &m)?;
        Ok(m)
    }

    /// The extracted text, at most `max_chars` of it.
    pub fn text(&self, material: &str, max_chars: usize) -> Result<String, StoreError> {
        let dir = self.material_dir(material)?;
        if !dir.join("meta.json").is_file() {
            return Err(StoreError::NotFound);
        }
        let text = std::fs::read_to_string(dir.join("text.txt"))?;
        Ok(text.chars().take(max_chars).collect())
    }

    /// Deletes Jay-S's copy and what was read from it (not the student's own file).
    pub fn delete(&self, material: &str) -> Result<(), StoreError> {
        let dir = self.material_dir(material)?;
        if !dir.join("meta.json").is_file() {
            return Err(StoreError::NotFound);
        }
        std::fs::remove_dir_all(dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, PathBuf) {
        let root = std::env::temp_dir().join(format!("jay-study-test-{}", id::new_id()));
        (Store::open(&root).unwrap(), root)
    }

    #[test]
    fn import_list_change_and_delete() {
        let (store, root) = temp_store();
        let file = root.join("prova.txt");
        let exam: String = (1..=6).map(|n| format!("Questão {n}\nQuanto é {n}+1?\na) {n}\nb) {}\nc) 0\nd) 9\ne) 7\n", n + 1)).collect();
        std::fs::write(&file, &exam).unwrap();
        let m = store.import(&file).unwrap();
        assert_eq!((m.name.as_str(), m.format, m.kind), ("prova.txt", Format::Text, MaterialKind::Exam));
        assert_eq!(store.materials(), vec![m.clone()]);
        assert_eq!(store.text(&m.id, 9).unwrap(), "Questão 1");
        let changed = store.set_kind(&m.id, MaterialKind::Both).unwrap();
        assert_eq!((changed.kind, changed.detected.kind), (MaterialKind::Both, MaterialKind::Exam));
        assert_eq!(store.material(&m.id).unwrap().kind, MaterialKind::Both);
        // The student's file is untouched; the copy goes away on delete.
        store.delete(&m.id).unwrap();
        assert!(store.materials().is_empty());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), exam);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hostile_ids_never_reach_the_disk() {
        let (store, root) = temp_store();
        // Something that must survive: a file next to the data folder.
        let outside = root.parent().unwrap().join(format!("{}-keep.txt", id::new_id()));
        std::fs::write(&outside, "keep").unwrap();
        for bad in ["..", "../..", "..\\..", outside.to_str().unwrap(), "CON", "", "materials", "0123456789ABCDEF0123456789ABCDEF"] {
            assert!(matches!(store.delete(bad), Err(StoreError::NotFound)), "{bad}");
            assert!(matches!(store.text(bad, 10), Err(StoreError::NotFound)), "{bad}");
            assert!(matches!(store.set_kind(bad, MaterialKind::Exam), Err(StoreError::NotFound)), "{bad}");
        }
        // A well-formed id that doesn't exist is not found either.
        assert!(matches!(store.delete(&id::new_id()), Err(StoreError::NotFound)));
        assert!(outside.exists() && root.join("materials").is_dir());
        std::fs::remove_file(outside).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_import_leaves_nothing() {
        let (store, root) = temp_store();
        let bad = root.join("x.pdf");
        std::fs::write(&bad, b"%PDF-1.7\nnot really").unwrap();
        assert!(matches!(store.import(&bad), Err(StoreError::Extract(ExtractError::Invalid(_)))));
        let program = root.join("x.exe");
        std::fs::write(&program, b"MZ\x90\0\x03\0\0\0\x04\0").unwrap();
        assert!(matches!(store.import(&program), Err(StoreError::Extract(ExtractError::Unsupported))));
        assert_eq!(std::fs::read_dir(root.join("materials")).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stray_or_planted_folders_are_skipped() {
        let (store, root) = temp_store();
        std::fs::create_dir_all(root.join("materials/not-an-id")).unwrap();
        std::fs::write(root.join("materials/not-an-id/meta.json"), "{}").unwrap();
        let fake = id::new_id();
        std::fs::create_dir_all(root.join("materials").join(&fake)).unwrap();
        std::fs::write(root.join("materials").join(&fake).join("meta.json"), "not json").unwrap();
        assert!(store.materials().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_write_replaces_and_leaves_no_tmp() {
        let (_store, root) = temp_store();
        let path = root.join("a.json");
        write_json(&path, &vec![1, 2]).unwrap();
        write_json(&path, &vec![3]).unwrap();
        assert_eq!(read_json::<Vec<i32>>(&path).unwrap(), vec![3]);
        assert_eq!(std::fs::read_dir(&root).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
