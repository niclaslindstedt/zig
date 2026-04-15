//! zig-core — workflow orchestration engine for AI coding agents.
//!
//! This crate provides the core library for parsing, validating, and executing
//! `.zwf` workflow files (and zipped `.zwfz` bundles). It powers the `zig` CLI.

pub mod config;
pub mod create;
pub mod docs;
pub mod error;
pub mod listen;
pub mod man;
pub mod manage;
pub mod memory;
pub mod pack;
pub mod paths;
pub mod prompt;
pub mod resources;
pub mod resources_manage;
pub mod run;
pub mod session;
pub mod storage;
pub mod update;
pub mod workflow;
