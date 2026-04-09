//! zig-core — workflow orchestration engine for AI coding agents.
//!
//! This crate provides the core library for parsing, validating, and executing
//! `.zug` workflow files. It powers the `zig` CLI.

pub mod create;
pub mod delete;
pub mod error;
pub mod man;
pub mod prompt;
pub mod run;
pub mod workflow;
