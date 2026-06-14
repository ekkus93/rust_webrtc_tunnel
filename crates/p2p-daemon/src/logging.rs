use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::DaemonError;
use p2p_core::LoggingConfig;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;

const REDACTED: &str = "<redacted>";

pub fn setup_logging(config: &LoggingConfig) -> Result<(), DaemonError> {
    let writer = build_writer(config)?;
    let builder = tracing_subscriber::fmt()
        .with_max_level(parse_level(&config.level)?)
        .with_target(false)
        .with_writer(writer);

    match config.format.as_str() {
        "json" => builder.json().try_init(),
        _ => builder.try_init(),
    }
    .map_err(|error| DaemonError::Logging(error.to_string()))
}

pub fn redact_secret(_value: &str) -> String {
    REDACTED.to_owned()
}

pub fn redact_sdp(config: &LoggingConfig, value: &str) -> String {
    if config.redact_sdp {
        format!("{REDACTED}:sdp:{}-bytes", value.len())
    } else {
        value.to_owned()
    }
}

pub fn redact_candidate(config: &LoggingConfig, value: &str) -> String {
    if config.redact_candidates {
        format!("{REDACTED}:candidate:{}-bytes", value.len())
    } else {
        value.to_owned()
    }
}

/// Build a diagnostic summary of an ICE candidate line for logging.
///
/// The candidate *type* (`host`/`srflx`/`prflx`/`relay`) and transport
/// (`udp`/`tcp`) reveal no address and are exactly what is needed to diagnose ICE
/// gathering and connectivity problems, so they are always included. The full
/// candidate line — which contains IP addresses and ports — is only appended when
/// candidate redaction is disabled (`logging.redact_candidates = false`), so the
/// "no candidate logging by default" policy is preserved.
pub fn candidate_log_summary(config: &LoggingConfig, value: &str) -> String {
    let typ = candidate_token_after(value, "typ").unwrap_or("unknown");
    // SDP candidate grammar: `candidate:<foundation> <component> <transport> ...`,
    // so the transport is the third whitespace-separated token.
    let transport = value.split_whitespace().nth(2).unwrap_or("unknown");
    format!("typ={typ} transport={transport} line={}", redact_candidate(config, value))
}

/// Return the token immediately following the first occurrence of `key` in a
/// whitespace-separated candidate line (e.g. the value after `typ`).
fn candidate_token_after<'a>(value: &'a str, key: &str) -> Option<&'a str> {
    let mut tokens = value.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == key {
            return tokens.next();
        }
    }
    None
}

fn build_writer(config: &LoggingConfig) -> Result<BoxMakeWriter, DaemonError> {
    let file = if config.file_logging {
        ensure_parent_exists(&config.log_file)?;
        Some(Arc::new(Mutex::new(
            OpenOptions::new().create(true).append(true).open(&config.log_file)?,
        )))
    } else {
        None
    };
    let write_stdout = config.stdout_logging;

    Ok(BoxMakeWriter::new(move || MultiWriter {
        stdout: write_stdout.then(io::stdout),
        file: file.as_ref().map(Arc::clone),
    }))
}

fn ensure_parent_exists(path: &Path) -> Result<(), DaemonError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn parse_level(level: &str) -> Result<LevelFilter, DaemonError> {
    match level {
        "trace" => Ok(LevelFilter::TRACE),
        "debug" => Ok(LevelFilter::DEBUG),
        "info" => Ok(LevelFilter::INFO),
        "warn" => Ok(LevelFilter::WARN),
        "error" => Ok(LevelFilter::ERROR),
        other => Err(DaemonError::Logging(format!("unsupported log level '{other}'"))),
    }
}

struct MultiWriter {
    stdout: Option<io::Stdout>,
    file: Option<Arc<Mutex<std::fs::File>>>,
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(stdout) = self.stdout.as_mut() {
            stdout.write_all(buf)?;
        }
        if let Some(file) = self.file.as_ref() {
            let mut file = file.lock().map_err(|_| io::Error::other("log file mutex poisoned"))?;
            file.write_all(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(stdout) = self.stdout.as_mut() {
            stdout.flush()?;
        }
        if let Some(file) = self.file.as_ref() {
            let mut file = file.lock().map_err(|_| io::Error::other("log file mutex poisoned"))?;
            file.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use p2p_core::LoggingConfig;

    use super::{candidate_log_summary, redact_candidate, redact_sdp, redact_secret};

    fn config() -> LoggingConfig {
        LoggingConfig {
            level: "info".to_owned(),
            format: "text".to_owned(),
            file_logging: false,
            stdout_logging: true,
            log_file: PathBuf::from("/tmp/log.txt"),
            redact_secrets: true,
            redact_sdp: true,
            redact_candidates: true,
            log_rotation: "none".to_owned(),
        }
    }

    #[test]
    fn secrets_are_always_redacted() {
        assert_eq!(redact_secret("super-secret"), "<redacted>");
    }

    #[test]
    fn sdp_is_redacted_by_default() {
        let redacted = redact_sdp(&config(), "v=0\r\na=candidate");
        assert!(redacted.starts_with("<redacted>:sdp:"));
        assert!(!redacted.contains("candidate"));
    }

    #[test]
    fn candidates_are_redacted_by_default() {
        let redacted = redact_candidate(&config(), "candidate:1 1 UDP 2122252543 192.0.2.1 12345");
        assert!(redacted.starts_with("<redacted>:candidate:"));
        assert!(!redacted.contains("192.0.2.1"));
    }

    #[test]
    fn candidate_summary_keeps_type_and_transport_but_redacts_address_by_default() {
        let summary = candidate_log_summary(
            &config(),
            "candidate:1 1 udp 2122252543 192.168.1.5 54321 typ host",
        );
        // Type and transport stay visible — that is the whole point of the summary.
        assert!(summary.contains("typ=host"), "summary was: {summary}");
        assert!(summary.contains("transport=udp"), "summary was: {summary}");
        // The address is still redacted while redact_candidates is on.
        assert!(!summary.contains("192.168.1.5"), "address leaked: {summary}");
    }

    #[test]
    fn candidate_summary_includes_address_when_redaction_disabled() {
        let mut config = config();
        config.redact_candidates = false;
        let summary = candidate_log_summary(
            &config,
            "candidate:2 1 udp 1686052607 203.0.113.7 54321 typ srflx raddr 192.168.1.5 rport 54321",
        );
        assert!(summary.contains("typ=srflx"), "summary was: {summary}");
        assert!(summary.contains("transport=udp"), "summary was: {summary}");
        assert!(summary.contains("203.0.113.7"), "address missing: {summary}");
    }

    #[test]
    fn candidate_summary_tolerates_malformed_lines() {
        let summary = candidate_log_summary(&config(), "garbage");
        assert!(summary.contains("typ=unknown"), "summary was: {summary}");
    }
}
