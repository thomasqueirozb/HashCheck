use thiserror::Error;

#[derive(Error, Debug)]
pub enum HashCheckError {
    #[error("new file found {0}")]
    Created(String),
    #[error("multiple entries for {0}")]
    Multiple(String),
    #[error("wrong sha256sum for {path:?}. (expected {expected:?}, found {found:?})")]
    WrongHash {
        path: String,
        expected: String,
        found: String,
    },
    // #[error("unknown error")]
    // Unknown,
}
