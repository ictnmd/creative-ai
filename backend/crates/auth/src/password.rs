//! Password hashing, verification, and strength validation utilities.

use common::AppResult;

/// Minimum password length.
const MIN_PASSWORD_LENGTH: usize = 8;

/// Characters considered letters (ASCII a-z and A-Z).
fn has_letter(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_alphabetic())
}

/// Characters considered digits (ASCII 0-9).
fn has_digit(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit())
}

/// Characters considered special (ASCII punctuation / symbols).
fn has_special(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_punctuation())
}

/// Hash a password using bcrypt.
///
/// Returns the bcrypt hash as a string. The hash includes the algorithm
/// version, cost factor, salt, and digest, making it safe to store
/// in a database.
///
/// # Errors
///
/// Returns an error if the hashing operation fails (e.g., invalid cost factor).
pub fn hash_password(password: &str) -> AppResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| common::AppError::Bcrypt(e.to_string()))
}

/// Verify a password against a bcrypt hash.
///
/// Returns `true` if the password matches the hash, `false` otherwise.
/// This operation is intentionally slow (bcrypt's work factor) to resist
/// brute-force and dictionary attacks.
pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// Validate password strength requirements.
///
/// A password is considered strong enough when it satisfies ALL of the
/// following criteria:
/// - At least 8 characters long
/// - Contains at least one letter (a-z, A-Z)
/// - Contains at least one digit (0-9)
/// - Contains at least one special character (!@#$%^&*... etc.)
///
/// # Errors
///
/// Returns `Err` with a descriptive message if the password does not
/// meet the strength requirements. The message lists which criteria
/// are unmet.
pub fn validate_password_strength(password: &str) -> AppResult<()> {
    let mut errors: Vec<String> = Vec::new();

    if password.len() < MIN_PASSWORD_LENGTH {
        errors.push(format!(
            "must be at least {} characters long",
            MIN_PASSWORD_LENGTH
        ));
    }

    if !has_letter(password) {
        errors.push("must contain at least one letter".to_string());
    }

    if !has_digit(password) {
        errors.push("must contain at least one number".to_string());
    }

    if !has_special(password) {
        errors.push("must contain at least one special character".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(common::AppError::Validation(errors.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "SecureP@ssw0rd!";
        let hash = hash_password(password).expect("hash should succeed");

        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrongpassword", &hash));
    }

    #[test]
    fn test_hash_is_different_each_time() {
        let password = "SecureP@ssw0rd!";
        let hash1 = hash_password(password).expect("first hash should succeed");
        let hash2 = hash_password(password).expect("second hash should succeed");

        // bcrypt hashes include a random salt, so successive hashes differ
        assert_ne!(hash1, hash2);
        // but both should verify correctly
        assert!(verify_password(password, &hash1));
        assert!(verify_password(password, &hash2));
    }

    #[test]
    fn test_validate_strength_valid() {
        assert!(validate_password_strength("SecureP@ss1").is_ok());
        assert!(validate_password_strength("Abc123!@#").is_ok());
        assert!(validate_password_strength("LongerP@ssw0rd123").is_ok());
    }

    #[test]
    fn test_validate_strength_too_short() {
        let result = validate_password_strength("Aa1!aaa");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("8 characters"));
    }

    #[test]
    fn test_validate_strength_no_letter() {
        let result = validate_password_strength("12345678!@");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("letter"));
    }

    #[test]
    fn test_validate_strength_no_digit() {
        let result = validate_password_strength("Abcdefgh!@");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("number"));
    }

    #[test]
    fn test_validate_strength_no_special() {
        let result = validate_password_strength("Abcd12345678");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("special character"));
    }

    #[test]
    fn test_validate_strength_multiple_failures() {
        let result = validate_password_strength("short");
        assert!(result.is_err());
        let err = result.unwrap_err();
        // "short" has letters but is too short, lacks digits and special chars.
        // Should mention multiple failures joined by semicolons.
        let msg = err.to_string();
        assert!(msg.contains("8 characters"));
        assert!(msg.contains("number"));
        assert!(msg.contains("special character"));
    }

    #[test]
    fn test_verify_invalid_hash() {
        // verify_password returns false for malformed hashes, not an error
        assert!(!verify_password("password", "not-a-bcrypt-hash"));
        assert!(!verify_password("password", ""));
    }
}
