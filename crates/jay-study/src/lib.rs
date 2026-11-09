//! Jay-S's engine, without Tauri (docs/ARQUITETURA.md, section 2): ids and the data folder, reading
//! attached files, telling content from exams. Tested with `cargo test` here.

pub mod ai;
pub mod cards;
pub mod classify;
pub mod exam;
pub mod extract;
pub mod generate;
pub mod id;
pub mod records;
pub mod store;

pub use classify::{Classification, MaterialKind};
pub use extract::{ExtractError, Extracted, Format, Warning};
pub use store::{Material, Store};
