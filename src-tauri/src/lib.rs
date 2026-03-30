//! TestForge - API and Web UI Testing Tool
//! 
//! This library provides the core functionality for TestForge,
//! including environment management, data tables, and secret encryption.

//!
//! # Architecture
//!
//! The library is organized into the following modules:
//!
//! - `error` - Error types and error handling
//! - `models` - Data models for database entities
//! - `repositories` - Database repository layer
//! - `services` - Business logic services
//! - `db` - Database connection and migrations
//! - `utils` - Utility functions (paths, crypto)
//! - `contracts` - IPC contracts for frontend communication
//! - `state` - Application state management

//!
//! # Quick Start
//!
//! ```rust,no_run
//! use testforge::{
//!     AppState, AppConfig,
//!     db::{Database, MigrationRunner},
//!     utils::paths::AppPaths,
//! };
//!
//! fn main() {
//!     // Initialize paths
//!     let paths = AppPaths::new("/path/to/app/data".into());
//!     paths.bootstrap().unwrap();
//!     
//!     // Initialize database
//!     let database = Database::new(paths.database_file()).unwrap();
//!     
//!     // Initialize secret service
//!     let secret_service = SecretService::new(paths.base.clone());
//!     secret_service.initialize().unwrap();
//!     
//!     // Create app state
//!     let app_state = AppState::new(database, secret_service, paths);
//!     
//!     // Use app_state...
//! }
//! ```

pub mod models;
pub mod repositories;
pub mod services;
pub mod db;
pub mod utils;
pub mod error;
pub mod contracts;
pub mod state;

// Re-export main types for convenience
pub use error::{AppError, AppResult, Result, TestForgeError};
pub use state::{AppState, AppConfig, RecordingState, RunState};
pub use db::{Database, DbConnection, MigrationRunner, MigrationResult};
pub use utils::paths::AppPaths;
