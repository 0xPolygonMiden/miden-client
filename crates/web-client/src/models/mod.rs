//!# Module Overview
//!
//! This module provides a set of structs and functionality that are exposed to JavaScript via
//! `wasm_bindgen`. These structs serve as wrappers around native objects from the acrss the miden
//! repositories. The goal is to provide a way to interact with these objects in a web context with
//! JavaScript, mimicking the same level of functionality and usability as when working with them in
//! Rust.
//!
//! ## Purpose
//!
//! This module is designed to enable developers to work with core objects and data structures used
//! in the miden client, directly from JavaScript in a browser environment. By exposing Rust-native
//! functionality via `wasm_bindgen`, it ensures that the web-based use of the miden client is as
//! close as possible to the Rust-native experience. These bindings allow the creation and
//! manipulation of important client structures, such as accounts, transactions, notes, and assets,
//! providing access to core methods and properties.
//!
//! ## Usage
//!
//! Each module provides Rust structs and methods that are exposed to JavaScript via `wasm_bindgen`.
//! These bindings allow developers to create and manipulate miden client objects in JavaScript,
//! while maintaining the same functionality and control as would be available in a pure Rust
//! environment.
//!
//! This makes it easy to build web-based applications that interact with the miden client, enabling
//! rich interaction with accounts, assets, and transactions directly from the browser.

#![allow(clippy::return_self_not_must_use)]

pub mod account;
pub mod account_builder;
pub mod account_code;
pub mod account_component;
pub mod account_delta;
pub mod account_header;
pub mod account_id;
pub mod account_id_anchor;
pub mod account_storage;
pub mod account_storage_mode;
pub mod account_type;
pub mod advice_inputs;
pub mod advice_map;
pub mod assembler;
pub mod asset_vault;
pub mod auth_secret_key;
pub mod block_header;
pub mod consumable_note_record;
pub mod executed_transaction;
pub mod felt;
pub mod fungible_asset;
pub mod input_note;
pub mod input_note_record;
pub mod input_note_state;
pub mod input_notes;
pub mod library;
pub mod merkle_path;
pub mod note;
pub mod note_assets;
pub mod note_details;
pub mod note_execution_hint;
pub mod note_execution_mode;
pub mod note_filter;
pub mod note_header;
pub mod note_id;
pub mod note_inclusion_proof;
pub mod note_inputs;
pub mod note_location;
pub mod note_metadata;
pub mod note_recipient;
pub mod note_script;
pub mod note_tag;
pub mod note_type;
pub mod output_note;
pub mod output_notes;
pub mod partial_note;
pub mod provers;
pub mod rpo256;
pub mod rpo_digest;
pub mod storage_map;
pub mod storage_slot;
pub mod sync_summary;
pub mod transaction_args;
pub mod transaction_filter;
pub mod transaction_id;
pub mod transaction_kernel;
pub mod transaction_record;
pub mod transaction_request;
pub mod transaction_result;
pub mod transaction_script;
pub mod transaction_script_inputs;
pub mod transaction_status;
pub mod transactions;
pub mod word;
