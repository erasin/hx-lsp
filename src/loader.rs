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
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.config_dir();
    path.push("helix"); // set editor ?
    path.push(d.to_string());
    path
}
