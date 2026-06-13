//! Shared imports for the transport unit-test suite.
//!
//! The grouped test files pull these in via `use super::support::*`. The
//! transport-internal symbols and crate types the tests exercise are re-exported
//! here so each group file needs only the single glob import. The per-group
//! fixtures (`codecs`, `sample_config`) live with their sole consumer.

pub(super) use std::path::PathBuf;

pub(super) use p2p_core::{
    AppConfig, BrokerConfig, BrokerTlsConfig, ForwardAnswerConfig, ForwardRule, HealthConfig,
    LoggingConfig, MsgId, NodeConfig, NodeRole, ReconnectConfig, SecurityConfig, TunnelConfig,
    WebRtcConfig,
};
pub(super) use p2p_core::{MessageType, SessionId};
pub(super) use p2p_crypto::{AuthorizedKeys, generate_identity};
pub(super) use rumqttc::mqttbytes::v4::{Publish, SubAck, SubscribeReasonCode};
pub(super) use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, Transport};

pub(super) use super::super::{
    EnvelopeFlags, InnerMessageBuilder, MqttSignalingTransport, OuterEnvelope, ReplayCache,
    ReplayStatus, SignalCodec, buffer_pending_own_topic_publish, build_mqtt_options,
    default_roots_tls_config, own_topic_publish_payload, signal_topic,
};
pub(super) use crate::{ErrorBody, MessageBody, OfferBody, SignalingError};
