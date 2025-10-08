use std::fmt;

use regex::Regex;

/// Error type returned when a session name is invalid.
#[derive(Debug)]
pub struct SessionNameError(String);

impl std::error::Error for SessionNameError {}

impl fmt::Display for SessionNameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validates a session name according to the rules:
///
/// - Must be between 1 and 30 characters long.
/// - Can only contain alphanumeric characters, underscores (`_`),
///   and hyphens (`-`).
///
/// # Errors
///
/// Returns a [`SessionNameError`] if the name is invalid.
///
/// # Examples
/// ```
/// # use tsman::validate_session_name;
/// assert!(validate_session_name("valid_name-123").is_ok());
/// assert!(validate_session_name("invalid name").is_err());
/// ```
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
