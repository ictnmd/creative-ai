//! # Auth Crate
//!
//! Authentication and authorization utilities including JWT handling,
//! password hashing, OAuth flows, and session management.

pub mod jwt;
pub mod password;

pub use jwt::{Claims, JwtService, RefreshClaims, TokenType};
pub use password::{hash_password, validate_password_strength, verify_password};
