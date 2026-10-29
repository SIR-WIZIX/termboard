use chrono;
use std::fmt::Display;

pub enum LogLevel {
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

pub struct Event {
    pub time: chrono::DateTime<chrono::Local>,
    pub text: String,
    pub level: LogLevel,
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{0:?} [{1}] - \"{2}\"", self.time, self.level, self.text)
    }
}

#[derive(Debug)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub user_name: Option<String>,
    pub user_id: Option<String>,
    //TODO decide on priority mechanism -> pub priority: i32,
    pub cpu_usage: f32, //in percents
    pub cpu_time_ms: u64,
    pub mem_used_bytes: u64, //in bytes
    pub executable_path: Option<std::path::PathBuf>,
    pub process_name: std::ffi::OsString,
    pub process_args: std::vec::Vec<std::ffi::OsString>, //while serialization we could store them
    //as strings in array
    pub active_time: std::time::Duration,
    pub start_time: std::time::SystemTime,
}

#[derive(Debug)]
pub struct SystemSnapshot {
    pub cpu_usage: f32,            //in percents
    pub mem_used_bytes: u64,       //in bytes
    pub mem_total_bytes: u64,      //in bytes
    pub mem_available_bytes: u64,  //in bytes
    pub swap_used_bytes: u64,      //in bytes
    pub swap_total_bytes: u64,     //in bytes
    pub swap_available_bytes: u64, //in bytes
    pub uptime: std::time::Duration,
    pub processes: std::vec::Vec<ProcessSnapshot>,
    pub snapshot_time: std::time::SystemTime,
}
