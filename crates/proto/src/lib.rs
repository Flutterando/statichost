use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u8 = 1;

pub const MAX_CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum TunnelMsg {
    Hello {
        version: u8,
        name: String,
    },
    Welcome {
        version: u8,
    },
    RequestStart {
        id: Uuid,
        method: String,
        path: String,
        query: Option<String>,
        headers: Vec<(String, String)>,
        has_body: bool,
    },
    BodyChunk {
        id: Uuid,
        seq: u32,
        data: ByteBuf,
    },
    BodyEnd {
        id: Uuid,
    },
    ResponseStart {
        id: Uuid,
        status: u16,
        headers: Vec<(String, String)>,
        has_body: bool,
    },
    ResponseError {
        id: Uuid,
        kind: ErrKind,
        msg: String,
    },
    Cancel {
        id: Uuid,
    },
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrKind {
    LocalRefused,
    LocalTimeout,
    Internal,
    BadGateway,
}

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("encode: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("decode: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

pub fn encode(msg: &TunnelMsg) -> Result<Vec<u8>, CodecError> {
    Ok(rmp_serde::to_vec_named(msg)?)
}

pub fn decode(bytes: &[u8]) -> Result<TunnelMsg, CodecError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

pub fn is_valid_name(name: &str) -> bool {
    let len = name.len();
    if !(1..=31).contains(&len) {
        return false;
    }
    let bytes = name.as_bytes();
    let first_ok = bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit();
    if !first_ok {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

pub const RESERVED_NAMES: &[&str] = &["www", "api", "admin", "dl", "install"];

pub fn is_reserved(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_request_start() {
        let id = Uuid::new_v4();
        let msg = TunnelMsg::RequestStart {
            id,
            method: "GET".into(),
            path: "/foo".into(),
            query: Some("x=1".into()),
            headers: vec![("host".into(), "bar.example".into())],
            has_body: false,
        };
        let bytes = encode(&msg).unwrap();
        let decoded = decode(&bytes).unwrap();
        match decoded {
            TunnelMsg::RequestStart { id: i, .. } => assert_eq!(i, id),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn name_validation() {
        assert!(is_valid_name("foo"));
        assert!(is_valid_name("foo-bar"));
        assert!(is_valid_name("foo123"));
        assert!(is_valid_name("1foo"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("-foo"));
        assert!(!is_valid_name("Foo"));
        assert!(!is_valid_name("foo_bar"));
        assert!(!is_valid_name(&"a".repeat(32)));
    }
}
