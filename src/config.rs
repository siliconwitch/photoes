use std::path::PathBuf;

use gtk::glib;

pub struct Config {
    pub library: Vec<PathBuf>,
    pub inbox: Vec<PathBuf>,
}

pub fn load() -> Config {
    let mut config = Config {
        library: Vec::new(),
        inbox: Vec::new(),
    };

    let text = std::fs::read_to_string(glib::user_config_dir().join("photoes/config"));

    let Ok(text) = text else {
        return config;
    };

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = value.trim();
        let directory = match value.strip_prefix("~/") {
            Some(below_home) => glib::home_dir().join(below_home),
            None => PathBuf::from(value),
        };

        match key.trim() {
            "library" => config.library.push(directory),
            "inbox" => config.inbox.push(directory),
            _ => {}
        }
    }

    config
}
