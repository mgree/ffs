pub struct Config {
    pub timestamp: std::time::SystemTime,
    pub uid: u32,
    pub gid: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            timestamp: std::time::SystemTime::now(),
            uid: 501,
            gid: 501,
        }
    }
}