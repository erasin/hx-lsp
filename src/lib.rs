// #![allow(dead_code, unused_imports)]

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub mod action;
pub mod clipboard;
pub mod colors;
pub mod encoding;
pub mod env;
pub mod errors;
pub mod fuzzy;
pub mod loader;
pub mod lsp;
pub mod parser;
pub mod snippet;
pub mod state;
pub mod variables;
