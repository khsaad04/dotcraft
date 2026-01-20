pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    ctx: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ctx)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self { ctx: value }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            ctx: value.to_string(),
        }
    }
}

impl From<flagge::Error> for Error {
    fn from(value: flagge::Error) -> Self {
        Self {
            ctx: value.to_string(),
        }
    }
}

pub enum LogLevel {
    Info,
    Warning,
    Error,
}

macro_rules! log {
    ($loglevel:ident, $($arg:tt)*) => {
        match LogLevel::$loglevel {
            LogLevel::Info => {
                print!("\x1b[0;32mINFO\x1b[0m: ");
                println!($($arg)*);
            }
            LogLevel::Warning => {
                print!("\x1b[0;33mWARNING\x1b[0m: ");
                println!($($arg)*);
            }
            LogLevel::Error => {
                eprint!("\x1b[0;31mERROR\x1b[0m: ");
                eprintln!($($arg)*);
            }
        }
    };
}
