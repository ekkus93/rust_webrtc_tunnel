use super::support::*;

#[test]
fn topic_generation_matches_spec() {
    let peer_id: p2p_core::PeerId = "answer-office".parse().expect("peer id");
    assert_eq!(signal_topic("p2ptunnel", &peer_id), "p2ptunnel/v1/nodes/answer-office/signal");
}

#[test]
fn transport_type_exists() {
    let _ = std::mem::size_of::<MqttSignalingTransport>();
}

#[test]
fn own_topic_publish_is_buffered_during_subscribe_handshake() {
    let own_topic = "p2ptunnel/v1/nodes/answer-office/signal";
    let event = Event::Incoming(Packet::Publish(Publish::new(
        own_topic,
        QoS::AtLeastOnce,
        b"hello".to_vec(),
    )));
    let mut pending = std::collections::VecDeque::new();

    assert!(buffer_pending_own_topic_publish(&event, own_topic, &mut pending));
    assert_eq!(pending.pop_front(), Some(b"hello".to_vec()));
}

#[test]
fn unrelated_events_are_not_buffered_as_pending_payloads() {
    let own_topic = "p2ptunnel/v1/nodes/answer-office/signal";
    let foreign_publish = Event::Incoming(Packet::Publish(Publish::new(
        "p2ptunnel/v1/nodes/offer-home/signal",
        QoS::AtLeastOnce,
        b"foreign".to_vec(),
    )));
    let suback = Event::Incoming(Packet::SubAck(SubAck::new(
        7,
        vec![SubscribeReasonCode::Success(QoS::AtLeastOnce)],
    )));
    let mut pending = std::collections::VecDeque::new();

    assert!(!buffer_pending_own_topic_publish(&foreign_publish, own_topic, &mut pending));
    assert!(!buffer_pending_own_topic_publish(&suback, own_topic, &mut pending));
    assert!(pending.is_empty());
}

#[test]
fn own_topic_publish_payload_extracts_only_matching_topic_payloads() {
    let own_topic = "p2ptunnel/v1/nodes/answer-office/signal";
    let matching_publish = Event::Incoming(Packet::Publish(Publish::new(
        own_topic,
        QoS::AtLeastOnce,
        b"match".to_vec(),
    )));
    let foreign_publish = Event::Incoming(Packet::Publish(Publish::new(
        "p2ptunnel/v1/nodes/offer-home/signal",
        QoS::AtLeastOnce,
        b"foreign".to_vec(),
    )));
    let suback = Event::Incoming(Packet::SubAck(SubAck::new(
        9,
        vec![SubscribeReasonCode::Success(QoS::AtLeastOnce)],
    )));

    assert_eq!(own_topic_publish_payload(&matching_publish, own_topic), Some(b"match".to_vec()));
    assert_eq!(own_topic_publish_payload(&foreign_publish, own_topic), None);
    assert_eq!(own_topic_publish_payload(&suback, own_topic), None);
}

#[tokio::test]
async fn poll_signal_payload_returns_buffered_payload_before_polling_network() {
    let options = MqttOptions::new("test-client", "localhost", 1883);
    let (client, event_loop) = AsyncClient::new(options, 10);
    let mut transport = MqttSignalingTransport {
        client,
        event_loop,
        own_topic: "p2ptunnel/v1/nodes/answer-office/signal".to_owned(),
        qos: QoS::AtLeastOnce,
        pending_payloads: std::collections::VecDeque::from([b"buffered".to_vec()]),
    };

    let payload = transport
        .poll_signal_payload()
        .await
        .expect("buffered payload should be returned without polling the network");

    assert_eq!(payload, Some(b"buffered".to_vec()));
    assert!(transport.pending_payloads.is_empty());
}
