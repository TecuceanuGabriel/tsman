use std::fmt;

use regex::Regex;

/// Invalid session name error - used as clap's `value_parser` error type.
#[derive(Debug)]
pub struct SessionNameError(String);

impl std::error::Error for SessionNameError {}

impl fmt::Display for SessionNameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Checks that a name is 1-30 chars and matches `[a-zA-Z0-9_-]`.
pub fn validate_session_name(name: &str) -> Result<String, SessionNameError> {
    let re = Regex::new(r"^[a-zA-Z0-9_-]{1,30}$").unwrap();
    if !re.is_match(name) {
        Err(SessionNameError(
            "Session name must be 1-30 characters long and only contain [a-zA-Z0-9_-]"
                .into(),
        ))
    } else {
        Ok(name.to_string())
    }
}
