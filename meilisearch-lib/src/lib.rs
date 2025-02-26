#[macro_use]
pub mod error;
pub mod options;

pub mod index;
pub mod index_controller;

pub use index_controller::{updates::store::Update, IndexController as MeiliSearch};

pub use milli;

mod compression;
mod document_formats;

use walkdir::WalkDir;

pub trait EnvSizer {
    fn size(&self) -> u64;
}

impl EnvSizer for heed::Env {
    fn size(&self) -> u64 {
        WalkDir::new(self.path())
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .fold(0, |acc, m| acc + m.len())
    }
}
