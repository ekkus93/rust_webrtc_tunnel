//! Transport unit-test suite.
//!
//! The shared imports live in [`support`]; the tests are grouped by concern:
//! the crypto codec / replay layer ([`codec`]), MQTT topic and own-topic event
//! handling ([`mqtt_events`]), and broker option / TLS construction
//! ([`mqtt_options`]).

mod support;

mod codec;
mod mqtt_events;
mod mqtt_options;
