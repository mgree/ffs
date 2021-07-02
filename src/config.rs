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
    pub allow_xattr: bool,
    pub keep_macos_xattr_file: bool,
    pub read_only: bool,
    pub input: Input,
    pub output: Output,
    pub mount: Option<PathBuf>,
    pub cleanup_mount: bool,
}

#[derive(Debug)]
pub enum Input {
    Stdin,
    File(PathBuf),
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Input::Stdin => write!(f, "<stdin>"),
            Input::File(file) => write!(f, "{}", file.display()),
        }
    }
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

    #[cfg(target_os = "macos")]
    fn platform_ignored_file(&self, s: &str) -> bool {
        !self.keep_macos_xattr_file && s.starts_with("._")
    }

    #[cfg(target_os = "linux")]
    fn platform_ignored_file(&self, s: &str) -> bool {
        false
    }

    pub fn ignored_file(&self, s: &str) -> bool {
        s == "." || s == ".." || self.platform_ignored_file(s)
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
            add_newlines: true,
            pad_element_names: true,
            base64: base64::STANDARD,
            try_decode_base64: false,
            allow_xattr: true,
            keep_macos_xattr_file: false,
            read_only: false,
            input: Input::Stdin,
            output: Output::Stdout,
            mount: None,
            cleanup_mount: false,
        }
    }
}
