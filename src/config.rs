#[derive(Debug)]
pub struct Config {
    pub timestamp: std::time::SystemTime,
    pub uid: u32,
    pub gid: u32,
    pub filemode: u16,
    pub dirmode: u16,
    pub add_newlines: bool,
    pub pad_element_names: bool,
    pub read_only: bool,
}

impl Config {
    pub fn normalize_name(&self, s: String) -> String {
        // inspired by https://en.wikipedia.org/wiki/Filename
        s.replace(".", "dot")
            .replace("/", "slash")
            .replace("\\", "backslash")
            .replace("?", "question")
            .replace("*", "star")
            .replace(":", "colon")
            .replace("\"", "dquote")
            .replace("<", "lt")
            .replace(">", "gt")
            .replace(",", "comma")
            .replace(";", "semi")
            .replace("=", "equal")
            .replace(" ", "space")
    }
    
}

impl Default for Config {
    fn default() -> Self {
        Config {
            timestamp: std::time::SystemTime::now(),
            uid: 501,
            gid: 501,
            filemode: 0o644,
            dirmode: 0o755,
            add_newlines: false,
            pad_element_names: true,
            read_only: false,
        }
    }
}