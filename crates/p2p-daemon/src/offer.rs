//! Offer-role daemon: binds local listeners, dials the configured remote peer,
//! runs a single multiplexed peer session at a time, and transparently attempts
//! ICE-restart reconnects before returning to the waiting-for-local-client steady
//! state. Startup/security failures are fatal; transport turbulence is recoverable.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use p2p_core::{
    AppConfig, ConfigError, DaemonState, FailureCode, ForwardOfferConfig, ForwardTable, SessionId,
};
use p2p_crypto::{AuthorizedKey, AuthorizedKeys, IdentityFile};
use p2p_signaling::{
    AckBody, AnswerBody, CloseBody, InnerMessage, InnerMessageBuilder, MessageBody,
    MqttSignalingTransport, OfferBody, OuterEnvelope, SignalCodec, SignalingError,
};
use p2p_tunnel::{OfferClient, OfferListener};
use p2p_webrtc::{IceConnectionState, WebRtcPeer};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};

use crate::DaemonError;
use crate::config::*;
use crate::messages::*;
use crate::predicates::*;
use crate::signaling::*;
use crate::status::*;
use crate::types::*;
#[cfg(any(test, debug_assertions))]
#[derive(Clone)]
pub struct OfferSessionTestHandle {
    pub session_id: SessionId,
    pub ice_state_injector: p2p_webrtc::IceStateInjectorForTests,
}

struct OfferSessionIo<'a> {
    client: OfferClient,
    accepted_clients: &'a mut mpsc::Receiver<Result<OfferClient, p2p_tunnel::TunnelError>>,
    remote: &'a AuthorizedKey,
    #[cfg(any(test, debug_assertions))]
    session_hook: Option<mpsc::UnboundedSender<OfferSessionTestHandle>>,
}

type OfferAcceptedClients<'a> =
    &'a mut mpsc::Receiver<Result<OfferClient, p2p_tunnel::TunnelError>>;

type OfferBridgeFuture<'a> = Pin<
    Box<
        dyn Future<Output = (Result<(), p2p_tunnel::TunnelError>, OfferAcceptedClients<'a>)>
            + Send
            + 'a,
    >,
>;

pub async fn run_offer_daemon(
    config: AppConfig,
    local_identity: IdentityFile,
    authorized_keys: AuthorizedKeys,
) -> Result<(), DaemonError> {
    let transport = MqttSignalingTransport::connect(&config)?;
    run_offer_daemon_with_transport(config, local_identity, authorized_keys, transport).await
}

pub async fn run_offer_daemon_with_transport<T: DaemonSignalingTransport>(
    config: AppConfig,
    local_identity: IdentityFile,
    authorized_keys: AuthorizedKeys,
    transport: T,
) -> Result<(), DaemonError> {
    #[cfg(any(test, debug_assertions))]
    {
        run_offer_daemon_with_transport_and_test_hook(
            config,
            local_identity,
            authorized_keys,
            transport,
            None,
        )
        .await
    }

    #[cfg(not(any(test, debug_assertions)))]
    {
        let mut transport = transport;
        run_offer_daemon_inner(config, local_identity, authorized_keys, &mut transport, None, None)
            .await
    }
}

/// Offer daemon entry point that streams live `DaemonStatus` to `status_sink` in
/// addition to the usual status-file behavior. Used by the Android runtime so the
/// UI reflects real daemon/connection state. Behaves identically to
/// [`run_offer_daemon`] otherwise.
pub async fn run_offer_daemon_with_status(
    config: AppConfig,
    local_identity: IdentityFile,
    authorized_keys: AuthorizedKeys,
    status_sink: tokio::sync::watch::Sender<DaemonStatus>,
) -> Result<(), DaemonError> {
    let mut transport = MqttSignalingTransport::connect(&config)?;
    run_offer_daemon_inner(
        config,
        local_identity,
        authorized_keys,
        &mut transport,
        None,
        Some(status_sink),
    )
    .await
}

#[cfg(any(test, debug_assertions))]
pub async fn run_offer_daemon_with_transport_and_test_hook<T: DaemonSignalingTransport>(
    config: AppConfig,
    local_identity: IdentityFile,
    authorized_keys: AuthorizedKeys,
    mut transport: T,
    session_hook: Option<mpsc::UnboundedSender<OfferSessionTestHandle>>,
) -> Result<(), DaemonError> {
    run_offer_daemon_inner(
        config,
        local_identity,
        authorized_keys,
        &mut transport,
        session_hook,
        None,
    )
    .await
}

async fn run_offer_daemon_inner<T: DaemonSignalingTransport>(
    config: AppConfig,
    local_identity: IdentityFile,
    authorized_keys: AuthorizedKeys,
    transport: &mut T,
    #[cfg(any(test, debug_assertions))] session_hook: Option<
        mpsc::UnboundedSender<OfferSessionTestHandle>,
    >,
    #[cfg(not(any(test, debug_assertions)))] _session_hook: Option<()>,
    status_sink: Option<tokio::sync::watch::Sender<DaemonStatus>>,
) -> Result<(), DaemonError> {
    validate_config_authorized_peers(&config, &authorized_keys)?;
    let codec = SignalCodec::new(
        &local_identity,
        &authorized_keys,
        config.security.max_clock_skew_secs,
        config.security.max_message_age_secs,
    );
    transport.subscribe_own_topic().await?;

    let status = match status_sink {
        Some(sink) => StatusWriter::with_sink(&config, sink),
        None => StatusWriter::new(&config),
    };
    let mut runtime = DaemonRuntimeState::new_connected();
    let mut ctx = RuntimeContext { config: &config, status: &status, runtime: &mut runtime };
    write_steady_state_status(&ctx).await;

    let (listeners, forward_statuses) = bind_offer_listeners(&config).await?;
    ctx.runtime.forward_statuses = forward_statuses;
    write_steady_state_status(&ctx).await;
    let mut accepted_clients = spawn_offer_accept_loops(listeners);
    let mut replay_cache = p2p_signaling::ReplayCache::new(config.security.replay_cache_size);
    let remote_peer_id = offer_remote_peer_id(&config)?;
    let remote = authorized_keys
        .get_by_peer_id(&remote_peer_id)
        .cloned()
        .ok_or_else(|| DaemonError::MissingAuthorizedPeer(remote_peer_id.to_string()))?;

    loop {
        write_steady_state_status(&ctx).await;
        tokio::select! {
            client = accepted_clients.recv() => {
                let client = client
                    .ok_or_else(|| DaemonError::Logging("offer accept loop stopped".to_owned()))??;
                tracing::info!("accepted local client and entering busy offer session state");
                let result =
                    run_offer_session(
                        &config,
                        &codec,
                        transport,
                        &mut ctx,
                        OfferSessionIo {
                            client,
                            accepted_clients: &mut accepted_clients,
                            remote: &remote,
                            #[cfg(any(test, debug_assertions))]
                            session_hook: session_hook.clone(),
                        },
                    )
                    .await;
                recover_daemon_after_session(&ctx, result).await;
                tracing::info!("offer daemon returned to waiting state");
            }
            payload = poll_idle_signal_payload(&mut ctx, transport) => {
                let Some(payload) = payload else {
                    continue;
                };

                tracing::debug!(
                    payload_len = payload.len(),
                    role = ?config.node.role,
                    "received signaling payload while waiting for local client"
                );

                let decode_result =
                    decode_idle_signaling_message(&codec, &payload, &mut replay_cache);
                let (envelope, message, sender) = match decode_result {
                    Ok(decoded) => decoded,
                    Err(error) => {
                        tracing::warn!(reason = %error, "rejecting signaling message");
                        continue;
                    }
                };

                tracing::debug!(
                    session_id = %message.session_id,
                    sender_peer_id = %sender.peer_id,
                    sender_kid = %envelope.sender_kid,
                    message_type = ?message.message_type,
                    role = ?config.node.role,
                    "decoded idle signaling message"
                );

                match &message.body {
                    MessageBody::Hello(_) => {
                        tracing::info!("received optional hello from {}", sender.peer_id);
                    }
                    _ => {
                        tracing::warn!("ignoring unexpected idle message {:?}", message.message_type);
                    }
                }
            }
        }
    }
}

async fn run_offer_session<'a, T: DaemonSignalingTransport>(
    config: &'a AppConfig,
    codec: &SignalCodec<'_>,
    transport: &mut T,
    ctx: &mut RuntimeContext<'_>,
    io: OfferSessionIo<'a>,
) -> Result<(), DaemonError> {
    let remote = io.remote;
    let peer = WebRtcPeer::new(&config.webrtc).await?;
    let session_id = SessionId::random();
    let mut session =
        ActiveSession::new(session_id, remote.clone(), peer, config.security.replay_cache_size);

    write_daemon_status(
        ctx,
        StatusSnapshot {
            active_session_id: Some(session.session_id),
            current_state: DaemonState::Negotiating,
        },
    )
    .await;

    tracing::debug!(
        session_id = %session.session_id,
        remote_peer_id = %remote.peer_id,
        "starting offer session and publishing hello"
    );

    publish_message(
        ctx,
        codec,
        transport,
        StatusSnapshot {
            active_session_id: Some(session.session_id),
            current_state: DaemonState::Negotiating,
        },
        None,
        remote,
        OutgoingSignal {
            message: build_hello_message(
                &config.node.peer_id,
                &remote.peer_id,
                session.session_id,
                "offer",
            ),
            response: false,
        },
    )
    .await?;

    let data_channel = session.peer.create_data_channel().await?;
    session.data_channel = Some(data_channel.clone());
    let offer_sdp = session.peer.create_offer().await?;
    tracing::debug!(
        session_id = %session.session_id,
        remote_peer_id = %remote.peer_id,
        sdp_len = offer_sdp.len(),
        "created local offer and publishing signaling offer"
    );
    publish_message(
        ctx,
        codec,
        transport,
        StatusSnapshot {
            active_session_id: Some(session.session_id),
            current_state: DaemonState::Negotiating,
        },
        Some(&mut session.signaling),
        remote,
        OutgoingSignal {
            message: InnerMessageBuilder::new(
                session.session_id,
                config.node.peer_id.clone(),
                session.remote_peer_id.clone(),
            )
            .build(MessageBody::Offer(OfferBody { sdp: offer_sdp })),
            response: false,
        },
    )
    .await?;

    #[cfg(any(test, debug_assertions))]
    if let Some(session_hook) = io.session_hook {
        let _ = session_hook.send(OfferSessionTestHandle {
            session_id: session.session_id,
            ice_state_injector: session.peer.ice_state_injector_for_tests(),
        });
    }

    let mut tick = interval(Duration::from_secs(1));
    let mut pending_client = Some(io.client);
    let mut accepted_clients = Some(io.accepted_clients);
    let mut offer_bridge: Option<OfferBridgeFuture<'a>> = None;
    let result = async {
        loop {
            if pending_client.is_some()
                && session.data_channel.as_ref().is_some_and(|channel| channel.is_open())
                && offer_bridge.is_none()
            {
                write_daemon_status(
                    ctx,
                    StatusSnapshot {
                        active_session_id: Some(session.session_id),
                        current_state: DaemonState::TunnelOpen,
                    },
                )
                .await;
                session.bridge_state = BridgeSessionState::Active;
                let channel =
                    session.data_channel.clone().ok_or(DaemonError::MissingDataChannel)?;
                let active_clients = accepted_clients.take().ok_or_else(|| {
                    DaemonError::Logging(
                        "offer session lost accepted-client queue while bridge was starting"
                            .to_owned(),
                    )
                })?;
                let client = pending_client.take().ok_or(DaemonError::MissingDataChannel)?;
                offer_bridge = Some(Box::pin(async move {
                    let result =
                        p2p_tunnel::run_multiplex_offer(channel, &config.tunnel, client, active_clients)
                            .await;
                    (result, active_clients)
                }));
            }
            tokio::select! {
                _ = tick.tick() => {
                    retry_pending_acks(
                        ctx,
                        transport,
                        StatusSnapshot {
                            active_session_id: Some(session.session_id),
                            current_state: session.state,
                        },
                        &mut session,
                    )
                    .await?;
                    if !session.signaling.ack_tracker.expired().is_empty() {
                        return Err(DaemonError::AckTimeout);
                    }
                }
                payload = poll_session_signal_payload(
                    ctx,
                    transport,
                    StatusSnapshot {
                        active_session_id: Some(session.session_id),
                        current_state: session.state,
                    },
                ) => {
                    if let Some(payload) = payload? {
                        process_offer_session_payload(
                            ctx,
                            codec,
                            transport,
                            remote,
                            &mut session,
                            &payload,
                        )
                        .await?;
                    }
                }
                candidate = session.peer.next_local_candidate() => {
                    if let Some(candidate) = candidate {
                        send_local_candidate(
                            ctx,
                            codec,
                            transport,
                            &mut session,
                            remote,
                            candidate,
                        )
                        .await?;
                    }
                }
                ice_state = session.peer.next_ice_state() => {
                    if let Some(ice_state) = ice_state {
                        if matches!(ice_state, IceConnectionState::Failed | IceConnectionState::Disconnected) {
                            offer_bridge = None;
                            if let Some(handle) = session.bridge_handle.take() {
                                handle.abort();
                            }
                            if session.bridge_state == BridgeSessionState::Active {
                                publish_message(
                                    ctx,
                                    codec,
                                    transport,
                                    StatusSnapshot {
                                        active_session_id: Some(session.session_id),
                                        current_state: session.state,
                                    },
                                    Some(&mut session.signaling),
                                    remote,
                                    OutgoingSignal {
                                        message: build_error_message(
                                            &config.node.peer_id,
                                            &session.remote_peer_id,
                                            session.session_id,
                                            FailureCode::IceFailed,
                                            "ice connection failed",
                                        ),
                                        response: false,
                                    },
                                ).await?;
                                // In v1 a live tunnel failure ends the current local client/session.
                                session.bridge_state = BridgeSessionState::Closed;
                                return Err(DaemonError::IceFailed(ice_state));
                            }
                            session.bridge_state = BridgeSessionState::Reconnecting;
                            if should_attempt_offer_reconnect(config, pending_client.is_some(), session.bridge_state)
                                && attempt_offer_reconnect(
                                    ctx,
                                    codec,
                                    transport,
                                    &mut session,
                                    remote,
                                )
                                .await?
                            {
                                session.bridge_state = BridgeSessionState::Pending;
                                continue;
                            }
                            publish_message(
                                ctx,
                                codec,
                                transport,
                                StatusSnapshot {
                                    active_session_id: Some(session.session_id),
                                    current_state: session.state,
                                },
                                Some(&mut session.signaling),
                                remote,
                                OutgoingSignal {
                                    message: build_error_message(
                                        &config.node.peer_id,
                                        &session.remote_peer_id,
                                        session.session_id,
                                        FailureCode::IceFailed,
                                        "ice connection failed",
                                    ),
                                    response: false,
                                },
                            ).await?;
                            session.bridge_state = BridgeSessionState::Closed;
                            return Err(DaemonError::IceFailed(ice_state));
                        }
                    }
                }
                bridge_result = async {
                    let handle = session.bridge_handle.as_mut().expect("guarded by select");
                    handle.await
                }, if session.bridge_handle.is_some() => {
                    let result = bridge_result
                        .map_err(|error| DaemonError::Logging(format!("bridge task join error: {error}")))?;
                    session.bridge_handle = None;
                    session.bridge_state = BridgeSessionState::Closed;
                    let _ = publish_message(
                        ctx,
                        codec,
                        transport,
                        StatusSnapshot {
                            active_session_id: Some(session.session_id),
                            current_state: session.state,
                        },
                        Some(&mut session.signaling),
                        remote,
                        OutgoingSignal {
                            message: InnerMessageBuilder::new(
                                session.session_id,
                                config.node.peer_id.clone(),
                                session.remote_peer_id.clone(),
                            )
                            .build(MessageBody::Close(CloseBody {
                                reason_code: "session_closed".to_owned(),
                                message: None,
                            })),
                            response: false,
                        },
                    )
                    .await;
                    result?;
                    return Ok(());
                }
                bridge_result = async {
                    let bridge = offer_bridge.as_mut().expect("guarded by select");
                    bridge.as_mut().await
                }, if offer_bridge.is_some() => {
                    offer_bridge = None;
                    let (bridge_result, returned_clients) = bridge_result;
                    accepted_clients = Some(returned_clients);
                    session.bridge_state = BridgeSessionState::Closed;
                    let _ = publish_message(
                        ctx,
                        codec,
                        transport,
                        StatusSnapshot {
                            active_session_id: Some(session.session_id),
                            current_state: session.state,
                        },
                        Some(&mut session.signaling),
                        remote,
                        OutgoingSignal {
                            message: InnerMessageBuilder::new(
                                session.session_id,
                                config.node.peer_id.clone(),
                                session.remote_peer_id.clone(),
                            )
                            .build(MessageBody::Close(CloseBody {
                                reason_code: "session_closed".to_owned(),
                                message: None,
                            })),
                            response: false,
                        },
                    )
                    .await;
                    bridge_result?;
                    return Ok(());
                }
            }
        }
    }
    .await;

    if let Err(error) = &result {
        tracing::warn!(reason = %error, session_id = %session.session_id, "offer session failed");
    }
    cleanup_active_session(&mut session).await;
    result
}

pub(crate) async fn handle_offer_session_message(
    message: &InnerMessage,
    session: &mut ActiveSession,
) -> Result<(), DaemonError> {
    match &message.body {
        MessageBody::Ack(AckBody { ack_msg_id }) => {
            session.signaling.ack_tracker.acknowledge(&p2p_core::MsgId::new(*ack_msg_id));
        }
        MessageBody::Answer(AnswerBody { sdp }) => {
            session.peer.apply_remote_answer(sdp).await?;
        }
        MessageBody::IceCandidate(body) => {
            session.peer.add_remote_candidate(candidate_from_body(body)).await?;
        }
        MessageBody::EndOfCandidates(_) => {}
        MessageBody::Close(body) => {
            return Err(DaemonError::RemoteClosed(body.reason_code.clone()));
        }
        MessageBody::Error(body) => {
            return Err(DaemonError::RemoteError(body.code.clone(), body.message.clone()));
        }
        _ => {
            tracing::warn!("ignoring unexpected message {:?}", message.message_type);
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn spawn_offer_accept_loop(
    listener: OfferListener,
) -> mpsc::Receiver<Result<OfferClient, p2p_tunnel::TunnelError>> {
    spawn_offer_accept_loops(vec![listener])
}

/// Bind a local TCP listener for each configured offer forward. Individual forwards
/// that fail to bind are recorded as `Error` (soft-fail) so one bad forward does not
/// take down the others; the per-forward outcomes are returned alongside the bound
/// listeners. It is still a daemon-level error if forwards are configured but none
/// could bind.
pub(crate) async fn bind_offer_listeners(
    config: &AppConfig,
) -> Result<(Vec<OfferListener>, Vec<ForwardRuntimeStatus>), DaemonError> {
    let table = ForwardTable::new(&config.forwards);
    let mut listeners = Vec::new();
    let mut statuses = Vec::new();
    for bind in table.offer_listeners().map_err(|error| {
        DaemonError::Config(ConfigError::InvalidConfig(format!(
            "invalid offer forward listeners: {error:?}"
        )))
    })? {
        let forward_id = bind.forward_id.to_string();
        let offer =
            ForwardOfferConfig { listen_host: bind.listen_host, listen_port: bind.listen_port };
        match OfferListener::bind(bind.forward_id, &offer).await {
            Ok(listener) => {
                tracing::info!(
                    forward_id = listener.forward_id(),
                    local_addr = %listener.local_addr()?,
                    "listening for local forward clients"
                );
                statuses.push(ForwardRuntimeStatus::listening(forward_id));
                listeners.push(listener);
            }
            Err(error) => {
                tracing::warn!(
                    forward_id = %forward_id,
                    reason = %error,
                    "failed to bind local forward listener; marking forward as error"
                );
                statuses.push(ForwardRuntimeStatus::error(forward_id, error.to_string()));
            }
        }
    }
    if !statuses.is_empty() && listeners.is_empty() {
        return Err(DaemonError::Config(ConfigError::InvalidConfig(
            "no offer forward listeners could be bound".to_owned(),
        )));
    }
    Ok((listeners, statuses))
}

fn spawn_offer_accept_loops(
    listeners: Vec<OfferListener>,
) -> mpsc::Receiver<Result<OfferClient, p2p_tunnel::TunnelError>> {
    let (tx, rx) = mpsc::channel(64);
    for listener in listeners {
        let tx = tx.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept_client().await {
                    Ok(accepted) => match tx.try_send(Ok(accepted)) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full(Ok(dropped))) => {
                            tracing::warn!(
                                forward_id = dropped.forward_id(),
                                "offer pending client queue is full; closing local client"
                            );
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => return,
                        Err(mpsc::error::TrySendError::Full(Err(_))) => {}
                    },
                    Err(error) => {
                        tracing::warn!(reason = %error, "offer accept loop hit recoverable listener error");
                        sleep(DAEMON_RUNTIME_RETRY_DELAY).await;
                    }
                }
            }
        });
    }
    drop(tx);
    rx
}

pub(crate) async fn process_offer_session_payload<T: DaemonSignalingTransport>(
    ctx: &mut RuntimeContext<'_>,
    codec: &SignalCodec<'_>,
    transport: &mut T,
    remote: &AuthorizedKey,
    session: &mut ActiveSession,
    payload: &[u8],
) -> Result<OfferSessionPayloadOutcome, DaemonError> {
    let (envelope, message, sender) = match codec.decode(
        payload,
        &mut session.signaling.replay_cache,
        Some(session.session_id),
    ) {
        Ok(decoded) => decoded,
        Err(error) => {
            if maybe_ack_duplicate_active_session_message(
                ctx, codec, transport, session, payload, &error,
            )
            .await?
            {
                return Ok(OfferSessionPayloadOutcome::Ignored);
            }
            tracing::warn!(
                reason = %error,
                session_id = %session.session_id,
                "rejecting signaling message during active offer session"
            );
            return Ok(OfferSessionPayloadOutcome::Ignored);
        }
    };
    if sender.peer_id != session.remote_peer_id {
        tracing::warn!(
            peer_id = %sender.peer_id,
            expected_peer_id = %session.remote_peer_id,
            "ignoring message from unexpected peer"
        );
        return Ok(OfferSessionPayloadOutcome::Ignored);
    }
    if message.message_type.requires_ack() {
        publish_message(
            ctx,
            codec,
            transport,
            StatusSnapshot {
                active_session_id: Some(session.session_id),
                current_state: session.state,
            },
            None,
            remote,
            OutgoingSignal {
                message: codec.build_ack(
                    remote.peer_id.clone(),
                    session.session_id,
                    envelope.msg_id,
                ),
                response: true,
            },
        )
        .await?;
    }
    handle_offer_session_message(&message, session).await?;
    Ok(OfferSessionPayloadOutcome::Handled)
}

pub(crate) async fn maybe_ack_duplicate_active_session_message<T: DaemonSignalingTransport>(
    ctx: &mut RuntimeContext<'_>,
    codec: &SignalCodec<'_>,
    transport: &mut T,
    session: &mut ActiveSession,
    payload: &[u8],
    error: &SignalingError,
) -> Result<bool, DaemonError> {
    let Some((duplicate_msg_id, ack_message)) = duplicate_active_session_ack_message(
        codec,
        session.session_id,
        &session.remote_authorized,
        &session.remote_peer_id,
        payload,
        error,
    ) else {
        return Ok(false);
    };

    if !session.duplicate_active_acks.record_if_new(duplicate_msg_id) {
        tracing::info!(
            session_id = %session.session_id,
            duplicate_msg_id = %duplicate_msg_id,
            role = ?ctx.config.node.role,
            "suppressing repeated duplicate active-session re-ack"
        );
        return Ok(true);
    }

    let envelope = OuterEnvelope::decode(payload)
        .map_err(|error| DaemonError::Signaling(SignalingError::Protocol(error.to_string())))?;

    publish_message(
        ctx,
        codec,
        transport,
        StatusSnapshot {
            active_session_id: Some(session.session_id),
            current_state: session.state,
        },
        None,
        &session.remote_authorized,
        OutgoingSignal { message: ack_message, response: true },
    )
    .await?;

    tracing::info!(
        session_id = %session.session_id,
        duplicate_msg_id = %envelope.msg_id,
        role = ?ctx.config.node.role,
        "re-acknowledged duplicate active-session signaling message"
    );
    Ok(true)
}

async fn attempt_offer_reconnect<T: DaemonSignalingTransport>(
    ctx: &mut RuntimeContext<'_>,
    codec: &SignalCodec<'_>,
    transport: &mut T,
    session: &mut ActiveSession,
    remote: &AuthorizedKey,
) -> Result<bool, DaemonError> {
    if !ctx.config.reconnect.enable_auto_reconnect {
        return Ok(false);
    }

    let max_attempts = ctx.config.reconnect.max_attempts;
    let mut attempt = 0;
    while should_continue_reconnect_attempt(max_attempts, attempt) {
        session.state = DaemonState::Backoff;
        write_daemon_status(
            ctx,
            StatusSnapshot {
                active_session_id: Some(session.session_id),
                current_state: session.state,
            },
        )
        .await;
        tokio::time::sleep(compute_backoff_delay(ctx.config, attempt)).await;

        if ctx.config.webrtc.enable_ice_restart && can_attempt_same_session_ice_restart(session) {
            session.state = DaemonState::IceRestarting;
            write_daemon_status(
                ctx,
                StatusSnapshot {
                    active_session_id: Some(session.session_id),
                    current_state: session.state,
                },
            )
            .await;
            if reconnect_with_offer(ctx, codec, transport, session, remote, true).await? {
                session.state = DaemonState::ConnectingDataChannel;
                return Ok(true);
            }
        }

        session.state = DaemonState::Renegotiating;
        write_daemon_status(
            ctx,
            StatusSnapshot {
                active_session_id: Some(session.session_id),
                current_state: session.state,
            },
        )
        .await;
        if reconnect_with_offer(ctx, codec, transport, session, remote, false).await? {
            session.state = DaemonState::ConnectingDataChannel;
            return Ok(true);
        }
        attempt = attempt.saturating_add(1);
    }

    Ok(false)
}

async fn reconnect_with_offer<T: DaemonSignalingTransport>(
    ctx: &mut RuntimeContext<'_>,
    codec: &SignalCodec<'_>,
    transport: &mut T,
    session: &mut ActiveSession,
    remote: &AuthorizedKey,
    ice_restart: bool,
) -> Result<bool, DaemonError> {
    if ice_restart {
        let offer_sdp = session.peer.create_offer_with_restart(true).await?;
        publish_message(
            ctx,
            codec,
            transport,
            StatusSnapshot {
                active_session_id: Some(session.session_id),
                current_state: session.state,
            },
            Some(&mut session.signaling),
            remote,
            OutgoingSignal {
                message: InnerMessageBuilder::new(
                    session.session_id,
                    ctx.config.node.peer_id.clone(),
                    session.remote_peer_id.clone(),
                )
                .build(MessageBody::Offer(OfferBody { sdp: offer_sdp })),
                response: false,
            },
        )
        .await?;
        wait_for_offer_reconnect_response(
            ctx,
            codec,
            transport,
            session,
            remote,
            Duration::from_secs(u64::from(ctx.config.reconnect.ice_restart_timeout_secs)),
        )
        .await
    } else {
        let peer = WebRtcPeer::new(&ctx.config.webrtc).await?;
        let data_channel = peer.create_data_channel().await?;
        let new_session_id = SessionId::random();
        let mut replacement = ActiveSession::new(
            new_session_id,
            remote.clone(),
            peer,
            ctx.config.security.replay_cache_size,
        );
        replacement.data_channel = Some(data_channel);
        let offer_sdp = replacement.peer.create_offer().await?;
        publish_message(
            ctx,
            codec,
            transport,
            StatusSnapshot {
                active_session_id: Some(replacement.session_id),
                current_state: session.state,
            },
            Some(&mut replacement.signaling),
            remote,
            OutgoingSignal {
                message: InnerMessageBuilder::new(
                    replacement.session_id,
                    ctx.config.node.peer_id.clone(),
                    replacement.remote_peer_id.clone(),
                )
                .build(MessageBody::Offer(OfferBody { sdp: offer_sdp })),
                response: false,
            },
        )
        .await?;
        if wait_for_offer_reconnect_response(
            ctx,
            codec,
            transport,
            &mut replacement,
            remote,
            Duration::from_secs(u64::from(ctx.config.reconnect.renegotiate_timeout_secs)),
        )
        .await?
        {
            let _ = session.peer.close().await;
            *session = replacement;
            return Ok(true);
        }
        Ok(false)
    }
}

async fn wait_for_offer_reconnect_response<T: DaemonSignalingTransport>(
    ctx: &mut RuntimeContext<'_>,
    codec: &SignalCodec<'_>,
    transport: &mut T,
    session: &mut ActiveSession,
    remote: &AuthorizedKey,
    timeout: Duration,
) -> Result<bool, DaemonError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut tick = interval(Duration::from_millis(250));
    loop {
        if session.data_channel.as_ref().is_some_and(|channel| channel.is_open()) {
            return Ok(true);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(false);
        }
        tokio::select! {
            _ = tick.tick() => {
                retry_pending_acks(
                    ctx,
                    transport,
                    StatusSnapshot {
                        active_session_id: Some(session.session_id),
                        current_state: session.state,
                    },
                    session,
                )
                .await?;
                if !session.signaling.ack_tracker.expired().is_empty() {
                    return Ok(false);
                }
            }
            payload = poll_session_signal_payload(
                ctx,
                transport,
                StatusSnapshot {
                    active_session_id: Some(session.session_id),
                    current_state: session.state,
                },
            ) => {
                if let Some(payload) = payload? {
                    process_offer_session_payload(
                        ctx,
                        codec,
                        transport,
                        remote,
                        session,
                        &payload,
                    )
                    .await?;
                    if session
                        .data_channel
                        .as_ref()
                        .is_some_and(|channel| channel.is_open())
                    {
                        return Ok(true);
                    }
                }
            }
            candidate = session.peer.next_local_candidate() => {
                if let Some(candidate) = candidate {
                    send_local_candidate(ctx, codec, transport, session, remote, candidate).await?;
                }
            }
            ice_state = session.peer.next_ice_state() => {
                if let Some(ice_state) = ice_state {
                    match ice_state {
                        IceConnectionState::Connected | IceConnectionState::Completed => return Ok(true),
                        IceConnectionState::Failed => return Ok(false),
                        _ => {}
                    }
                }
            }
        }
    }
}
