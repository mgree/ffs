use fuser::FileType;
use std::path::PathBuf;

use super::format::Format;

#[derive(Debug)]
pub struct Config {
    pub input_format: Format,
    pub output_format: Format,
    pub uid: u32,
    pub gid: u32,
    pub filemode: u16,
    pub dirmode: u16,
    pub add_newlines: bool,
    pub pad_element_names: bool,
    pub base64: base64::Config,
    pub try_decode_base64: bool,
    pub read_only: bool,
    pub output: Output,
}

#[derive(Debug)]
pub enum Output {
    Quiet,
    Stdout,
    File(PathBuf),
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

    /// Determines the default mode of a file
    pub fn mode(&self, kind: FileType) -> u16 {
        if kind == FileType::Directory {
            self.dirmode
        } else {
            self.filemode
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_format: Format::Json,
            output_format: Format::Json,
            uid: 501,
            gid: 501,
            filemode: 0o644,
            dirmode: 0o755,
            add_newlines: false,
            pad_element_names: true,
            base64: base64::STANDARD,
            try_decode_base64: false,
            read_only: false,
            output: Output::Stdout,
        }
    }
}
