use bytes::{BufMut, BytesMut};
use p2p_core::{ACTIVE_STREAM_ID, FRAME_VERSION, FailureCode, TunnelFrameType};

use crate::TunnelError;

const HEADER_LEN: usize = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TunnelFrame {
    pub version: u8,
    pub frame_type: TunnelFrameType,
    pub stream_id: u32,
    pub payload: Vec<u8>,
}

impl TunnelFrame {
    pub fn new(frame_type: TunnelFrameType, payload: Vec<u8>) -> Self {
        Self { version: FRAME_VERSION, frame_type, stream_id: ACTIVE_STREAM_ID, payload }
    }

    pub fn open() -> Self {
        Self::new(TunnelFrameType::Open, Vec::new())
    }

    pub fn data(payload: Vec<u8>) -> Self {
        Self::new(TunnelFrameType::Data, payload)
    }

    pub fn close() -> Self {
        Self::new(TunnelFrameType::Close, Vec::new())
    }

    pub fn ping(payload: Vec<u8>) -> Self {
        Self::new(TunnelFrameType::Ping, payload)
    }

    pub fn pong(payload: Vec<u8>) -> Self {
        Self::new(TunnelFrameType::Pong, payload)
    }

    pub fn error(code: FailureCode) -> Self {
        Self::new(TunnelFrameType::Error, code.as_str().as_bytes().to_vec())
    }
}

#[derive(Debug, Default)]
pub struct TunnelFrameCodec;

impl TunnelFrameCodec {
    pub fn encode(frame: &TunnelFrame) -> Result<Vec<u8>, TunnelError> {
        if frame.version != FRAME_VERSION {
            return Err(TunnelError::unsupported_version(frame.version));
        }
        if frame.stream_id != ACTIVE_STREAM_ID {
            return Err(TunnelError::unsupported_stream_id(frame.stream_id));
        }

        let payload_len = u32::try_from(frame.payload.len())
            .map_err(|_| TunnelError::InvalidFrame("payload exceeds u32 length".to_owned()))?;

        let mut buffer = BytesMut::with_capacity(HEADER_LEN + frame.payload.len());
        buffer.put_u8(frame.version);
        buffer.put_u8(frame.frame_type as u8);
        buffer.put_u32(frame.stream_id);
        buffer.put_u32(payload_len);
        buffer.extend_from_slice(&frame.payload);
        Ok(buffer.to_vec())
    }

    pub fn decode(encoded: &[u8]) -> Result<TunnelFrame, TunnelError> {
        if encoded.len() < HEADER_LEN {
            return Err(TunnelError::TruncatedFrame);
        }

        let version = encoded[0];
        if version != FRAME_VERSION {
            return Err(TunnelError::unsupported_version(version));
        }

        let frame_type = tunnel_frame_type_from_u8(encoded[1])
            .ok_or(TunnelError::UnknownFrameType(encoded[1]))?;
        let stream_id = u32::from_be_bytes([encoded[2], encoded[3], encoded[4], encoded[5]]);
        if stream_id != ACTIVE_STREAM_ID {
            return Err(TunnelError::unsupported_stream_id(stream_id));
        }

        let payload_len =
            u32::from_be_bytes([encoded[6], encoded[7], encoded[8], encoded[9]]) as usize;
        if encoded.len() != HEADER_LEN + payload_len {
            return Err(TunnelError::InvalidFrame(format!(
                "payload length mismatch: header says {payload_len}, frame has {} payload bytes",
                encoded.len().saturating_sub(HEADER_LEN)
            )));
        }

        Ok(TunnelFrame { version, frame_type, stream_id, payload: encoded[HEADER_LEN..].to_vec() })
    }
}

fn tunnel_frame_type_from_u8(value: u8) -> Option<TunnelFrameType> {
    match value {
        0 => Some(TunnelFrameType::Open),
        1 => Some(TunnelFrameType::Data),
        2 => Some(TunnelFrameType::Close),
        3 => Some(TunnelFrameType::Error),
        4 => Some(TunnelFrameType::Ping),
        5 => Some(TunnelFrameType::Pong),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use p2p_core::{ACTIVE_STREAM_ID, FRAME_VERSION, TunnelFrameType};

    use super::{TunnelFrame, TunnelFrameCodec};
    use crate::TunnelError;

    #[test]
    fn frame_round_trip() {
        let frame = TunnelFrame::data(vec![1, 2, 3, 4]);
        let encoded = TunnelFrameCodec::encode(&frame).expect("frame should encode");
        let decoded = TunnelFrameCodec::decode(&encoded).expect("frame should decode");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn reject_invalid_frame_lengths() {
        let mut encoded = TunnelFrameCodec::encode(&TunnelFrame::data(vec![1, 2, 3]))
            .expect("frame should encode");
        encoded.truncate(encoded.len() - 1);
        assert!(matches!(TunnelFrameCodec::decode(&encoded), Err(TunnelError::InvalidFrame(_))));
    }

    #[test]
    fn reject_unsupported_stream_ids() {
        let mut encoded =
            TunnelFrameCodec::encode(&TunnelFrame::data(vec![9])).expect("frame should encode");
        encoded[2..6].copy_from_slice(&(ACTIVE_STREAM_ID + 1).to_be_bytes());
        assert!(matches!(
            TunnelFrameCodec::decode(&encoded),
            Err(TunnelError::UnsupportedStreamId { .. })
        ));
    }

    #[test]
    fn reject_unsupported_versions() {
        let mut encoded =
            TunnelFrameCodec::encode(&TunnelFrame::data(vec![9])).expect("frame should encode");
        encoded[0] = FRAME_VERSION + 1;
        assert!(matches!(
            TunnelFrameCodec::decode(&encoded),
            Err(TunnelError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn reject_unknown_frame_types() {
        let mut encoded =
            TunnelFrameCodec::encode(&TunnelFrame::data(vec![9])).expect("frame should encode");
        encoded[1] = 99;
        assert!(matches!(
            TunnelFrameCodec::decode(&encoded),
            Err(TunnelError::UnknownFrameType(99))
        ));
    }

    #[test]
    fn preserve_open_frame_structure() {
        let frame = TunnelFrame::open();
        let encoded = TunnelFrameCodec::encode(&frame).expect("frame should encode");
        assert_eq!(encoded[0], FRAME_VERSION);
        assert_eq!(encoded[1], TunnelFrameType::Open as u8);
        assert_eq!(
            u32::from_be_bytes([encoded[2], encoded[3], encoded[4], encoded[5]]),
            ACTIVE_STREAM_ID
        );
    }
}
