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
