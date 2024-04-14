// loader

// .config/helix/snippets
// setting config dir
// project/.helix/snippets

use std::path::PathBuf;

use etcetera::{choose_base_strategy, BaseStrategy};

/// Dirs ...
pub enum Dirs {
    Snippets,
    Actions,
}

impl Dirs {
    pub fn to_str(&self) -> String {
        match &self {
            Dirs::Snippets => "snippets".to_owned(),
            Dirs::Actions => "actions".to_owned(),
        }
    }
}

pub fn config_dir(d: Dirs) -> PathBuf {
    let strategy = choose_base_strategy().expect("Unable to find the config directory!");
    let mut path = strategy.config_dir();
    path.push("helix"); // set editor ?
    path.push(d.to_str());
    path
}
