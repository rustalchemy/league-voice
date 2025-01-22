use std::fmt::Display;

#[derive(Debug)]
pub struct DecodeError(pub String);

impl Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to decode packet: {}", self.0)
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_display_decode_error() {
        let error = DecodeError("Buffer is too small".to_string());
        assert_eq!(
            error.to_string(),
            "Failed to decode packet: Buffer is too small"
        );
    }
}
