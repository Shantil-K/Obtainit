//! lifter-core: the engine behind Obtainit.
//!
//! This is the download / scrape / extract machinery from
//! [lifter](https://github.com/cjrh/lifter) (AGPL-3.0), reorganised so a GUI
//! can call each step on its own instead of driving everything from a
//! `lifter.config` INI file:
//!
//! * [`http`]    – lifter's HTTP agent with its retry/backoff loop, plus
//!   streaming downloads with progress.
//! * [`scrape`]  – lifter's two discovery methods: `api_json` (JSONPath) and
//!   HTML (CSS selectors + an anchor-text regex).
//! * [`assets`]  – lifter's platform scoring used to pick the right release
//!   asset for this OS/CPU, and its anchor-regex generator.
//! * [`archive`] – lifter's archive extraction targets (singular/plural),
//!   extended with whole-archive extraction and more formats.
//! * [`version`] – version comparison (lifter's long-standing TODO).
//! * [`config`]  – read and write `lifter.config` files.

pub mod archive;
pub mod assets;
pub mod config;
pub mod http;
pub mod scrape;
pub mod version;

pub use anyhow::{Error, Result};
