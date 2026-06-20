//! Multiplex unit-test suite.
//!
//! The shared fixtures live in [`support`]; the tests are grouped by concern into
//! the sibling modules below.

mod support;

mod answer_frames;
mod end_to_end;
mod heartbeat;
mod offer_frames;
mod probe;
mod state;
mod stream_runtime;
