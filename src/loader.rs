// loader

// .config/helix/snippets
// setting config dir
// project/.helix/snippets

use std::path::PathBuf;

use etcetera::{BaseStrategy, choose_base_strategy};

/// Dirs ...
pub enum Dirs {
    Snippets,
    Actions,
}

impl std::fmt::Display for Dirs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Dirs::Snippets => "snippets",
                Dirs::Actions => "actions",
            }
        )
    }
}

pub fn config_dir(d: Dirs) -> PathBuf {
    let strategy = match choose_base_strategy() {
        Ok(s) => s,
        Err(_) => return PathBuf::new(),
    };
    let mut path = strategy.config_dir();
    path.push("helix");
    path.push(d.to_string());
    path
}
