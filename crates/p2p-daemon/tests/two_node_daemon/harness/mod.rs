//! Shared test harness for the two-node daemon integration tests, split by concern:
//! the in-memory faulting [`transport`], [`config`]/identity builders, [`net`] TCP
//! client and echo helpers, [`status`]-file polling/predicates, [`signal`]-trace
//! decoding, and the [`session`] scenario driver. Re-exported flat so test modules
//! can `use crate::harness::*`.

mod config;
mod net;
mod session;
mod signal;
mod status;
mod transport;

pub(crate) use config::*;
pub(crate) use net::*;
pub(crate) use session::*;
pub(crate) use signal::*;
pub(crate) use status::*;
pub(crate) use transport::*;
