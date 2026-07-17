use std::fmt::Display;

enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

struct Event {
    time: std::time::Instant,
    text: String,
    level: LogLevel,
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{0:?} [{1}] - \"{2}\"", self.time, self.level, self.text)
    }
}
