//! Jay-S's engine, without Tauri (docs/ARQUITETURA.md, section 2): ids and the data folder, reading
//! attached files, telling content from exams. Tested with `cargo test` here.

pub mod ai;
pub mod classify;
pub mod extract;
pub mod id;
pub mod store;

pub use classify::{Classification, MaterialKind};
pub use extract::{ExtractError, Extracted, Format, Warning};
pub use store::{Material, Store};
