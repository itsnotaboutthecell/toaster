//! Test aggregator for the `editor` module.
//!
//! These tests were previously defined inline in `editor/mod.rs`, which
//! pushed that file past 2300 lines. They now live as sibling files under
//! `editor/tests/` per the monolith-split plan; behavior is unchanged.

#![cfg(test)]

mod common;

mod basic;
mod dual_track_regression;
mod local_llm;
mod precision_eval;
mod seams;
