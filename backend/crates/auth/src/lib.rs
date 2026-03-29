//! # Auth Crate
//!
//! Authentication and authorization utilities including JWT handling,
//! password hashing, OAuth flows, and session management.

pub mod jwt;
pub mod oauth;
pub mod password;

pub use jwt::{Claims, JwtService, RefreshClaims, TokenType};
pub use oauth::{
    exchange_code_github, exchange_code_google, generate_pkce_pair, generate_state, get_github_user,
    get_google_user, github_auth_url, google_auth_url, validate_state, GitHubUserInfo,
    GoogleUserInfo, OAuthStateStore, Provider,
};
pub use password::{hash_password, validate_password_strength, verify_password};
