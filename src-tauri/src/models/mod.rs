//! Domain models for TestForge
//! 
//! This module contains all domain models used in the application.

pub mod environment;
pub mod environment_variable;
pub mod data_table;
pub mod data_table_row;

pub use environment::{Environment, EnvironmentType};
pub use environment_variable::{EnvironmentVariable, VariableType};
pub use data_table::{ColumnDefinition, DataTable};
pub use data_table_row::DataTableRow;
