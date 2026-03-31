//! LLM Providers
//!
//! This crate provides provider implementations for various LLM APIs.
//! The architecture separates authentication from API wire formats:
//!
//! - [`Auth`] trait: Base URL and authentication headers
//! - [`ApiSpec`] trait: Request/response wire format and endpoint path
//! - [`Provider`]: Generic composition of Auth + ApiSpec implementing [`ModelProvider`]
//!
//! ## Usage
//!
//! Use model enums to create configured providers:
//!
//! ```ignore
//! use naaf_providers::openai::OpenAiModel;
//! use naaf_providers::opencode_go::{OpenCodeGoModel, OpenCodeGoProvider};
//!
//! let gpt4 = OpenAiModel::Gpt4.provider("your-api-key");
//! let glm5 = OpenCodeGoModel::Glm5.provider("your-api-key");
//! ```

pub mod api;
pub mod auth;
pub mod provider;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "opencode-go")]
pub mod opencode_go;

pub use api::ApiSpec;
pub use auth::Auth;
pub use provider::Provider;
