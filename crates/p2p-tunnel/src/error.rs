use p2p_core::{ACTIVE_STREAM_ID, FRAME_VERSION, FailureCode, TunnelFrameType};

#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("unsupported tunnel frame version {actual}; expected {expected}")]
    UnsupportedVersion { actual: u8, expected: u8 },
    #[error("unsupported tunnel stream id {actual}; expected {expected}")]
    UnsupportedStreamId { actual: u32, expected: u32 },
    #[error("unknown tunnel frame type {0}")]
    UnknownFrameType(u8),
    #[error("truncated tunnel frame")]
    TruncatedFrame,
    #[error("invalid tunnel frame: {0}")]
    InvalidFrame(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("webrtc error: {0}")]
    WebRtc(#[from] p2p_webrtc::WebRtcError),
    #[error("offer listener is busy")]
    Busy,
    #[error("data channel closed")]
    DataChannelClosed,
    #[error("unexpected tunnel frame {0:?}")]
    UnexpectedFrame(TunnelFrameType),
    #[error("remote tunnel failure: {}", code.as_str())]
    RemoteFailure { code: FailureCode, detail: Option<String> },
}

impl TunnelError {
    pub fn unsupported_version(actual: u8) -> Self {
        Self::UnsupportedVersion { actual, expected: FRAME_VERSION }
    }

    pub fn unsupported_stream_id(actual: u32) -> Self {
        Self::UnsupportedStreamId { actual, expected: ACTIVE_STREAM_ID }
    }
}
