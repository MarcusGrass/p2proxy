use anyhow::bail;
use iroh::endpoint::VarInt;
use std::borrow::Borrow;
use std::fmt::Display;

pub const ALPN: &[u8] = b"p2proxy_proto";

pub const HEADER_LENGTH: usize = 16;

pub const PING: &[u8; HEADER_LENGTH] = b"PINGPINGPINGPING";

pub const DEFAULT_ROUTE: &[u8; HEADER_LENGTH] = b"9999999999999999";

pub const QUIC_OK_ERROR_CODE: VarInt = VarInt::from_u32(0);
pub const GENERIC_QUIC_ERROR_CODE: VarInt = VarInt::from_u32(1);
pub const FORBIDDEN_QUIC_ERROR_CODE: VarInt = VarInt::from_u32(2);

#[repr(transparent)]
#[derive(Eq, PartialEq, Hash, Debug, Clone, Default)]
pub struct ServerPortMapString(String);

impl Display for ServerPortMapString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ServerPortMapString {
    pub fn try_new(mut s: String) -> anyhow::Result<Self> {
        if s.len() > HEADER_LENGTH {
            if s.is_char_boundary(HEADER_LENGTH) {
                bail!("ServerPortMapString {s} is too long and cannot be truncated");
            }
            s.truncate(HEADER_LENGTH);
        }
        if s.len() != HEADER_LENGTH {
            let delta = HEADER_LENGTH - s.len();
            s.push_str(&"0".repeat(delta));
        }
        Ok(Self(s))
    }

    #[inline]
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn as_bytes(&self) -> &[u8; HEADER_LENGTH] {
        self.0.as_bytes().try_into().unwrap()
    }

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Borrow<str> for ServerPortMapString {
    #[inline]
    fn borrow(&self) -> &str {
        self.0.as_str()
    }
}
