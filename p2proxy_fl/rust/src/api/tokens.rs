use flutter_rust_bridge::frb;
use iroh_base::{NodeId, SecretKey};
use p2proxy_lib::display_chain;
use std::str::FromStr;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UserDefinedNode {
    pub(crate) node_id: NodeId,
    pub(crate) name: Option<String>,
}

impl UserDefinedNode {
    #[frb(sync)]
    pub fn try_new(node_id: &str, name: Option<String>) -> Result<Self, String> {
        let node_id = NodeId::from_str(node_id)
            .map_err(|e| format!("invalid node id: {}", display_chain(&e)))?;
        Ok(Self { node_id, name })
    }

    #[inline]
    #[frb(sync)]
    pub fn address(&self) -> String {
        self.node_id.to_string()
    }

    #[inline]
    #[frb(sync)]
    pub fn display_label(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.address().get(..8).map(|s| format!("{s}...")))
            .unwrap_or_else(|| self.address())
    }

    pub fn add_and_serialize(
        mut many: Vec<Self>,
        tgt: &Self,
    ) -> Result<(String, Vec<Self>), String> {
        many.push(tgt.clone());
        Self::serialize_many(&many).map(|s| (s, many))
    }

    fn serialize_many(many: &[Self]) -> Result<String, String> {
        serde_json::to_string(many).map_err(|e| {
            format!(
                "failed to serialize user defined nodes: {}",
                display_chain(&e)
            )
        })
    }

    pub fn remove_and_serialize_if_present(
        mut many: Vec<Self>,
        tgt: &Self,
    ) -> Result<(Option<String>, Vec<Self>), String> {
        let mut removed = false;
        many.retain(|v| {
            if v == tgt {
                removed = true;
                false
            } else {
                true
            }
        });
        if removed {
            Self::serialize_many(&many).map(|s| (Some(s), many))
        } else {
            Ok((None, many))
        }
    }

    pub fn deserialize_many(s: &str) -> Result<Vec<Self>, String> {
        serde_json::from_str(s).map_err(|e| {
            format!(
                "failed to deserialize user defined nodes: {}",
                display_chain(&e)
            )
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UserDefinedKey {
    pub(crate) public_key: NodeId,
    pub(crate) private_key: SecretKey,
    pub(crate) name: Option<String>,
}

impl Eq for UserDefinedKey {}

impl PartialEq for UserDefinedKey {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.private_key.to_bytes() == other.private_key.to_bytes()
    }
}

impl UserDefinedKey {
    #[frb(sync)]
    pub fn try_new(raw_hex: &str, name: Option<String>) -> Result<Self, String> {
        let privk_bytes = hex::decode(raw_hex).map_err(|e| format!("invalid private key: {e}"))?;
        let privk_bytes: [u8; 32] = privk_bytes
            .try_into()
            .map_err(|_| "invalid private key, unexpected length")?;
        let privk = SecretKey::from_bytes(&privk_bytes);
        let pubk = privk.public();
        Ok(Self {
            public_key: pubk,
            private_key: privk,
            name,
        })
    }

    #[inline]
    #[frb(sync)]
    pub fn generate_with_name(name: Option<String>) -> Self {
        let private_key = p2proxy_client::generate_secret_key();
        let public_key = private_key.public();
        Self {
            public_key,
            private_key,
            name,
        }
    }

    #[inline]
    #[frb(sync)]
    pub fn generate_key() -> String {
        let private_key = p2proxy_client::generate_secret_key();
        hex::encode(&private_key.to_bytes())
    }

    #[inline]
    #[frb(sync)]
    pub fn public_key_hex(&self) -> String {
        self.public_key.to_string()
    }

    #[inline]
    #[frb(sync)]
    pub fn private_key_hex(&self) -> String {
        hex::encode(&self.private_key.to_bytes())
    }

    #[inline]
    #[frb(sync)]
    pub fn display_label(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.public_key_hex().get(..8).map(|s| format!("{s}...")))
            .unwrap_or_else(|| self.public_key_hex())
    }

    pub fn add_and_serialize(
        mut many: Vec<Self>,
        tgt: &Self,
    ) -> Result<(String, Vec<Self>), String> {
        many.push(tgt.clone());
        Self::serialize_many(&many).map(|s| (s, many))
    }

    fn serialize_many(many: &[Self]) -> Result<String, String> {
        serde_json::to_string(many).map_err(|e| {
            format!(
                "failed to serialize user defined nodes: {}",
                display_chain(&e)
            )
        })
    }

    pub fn remove_and_serialize_if_present(
        mut many: Vec<Self>,
        tgt: &Self,
    ) -> Result<(Option<String>, Vec<Self>), String> {
        let mut removed = false;
        many.retain(|v| {
            if v == tgt {
                removed = true;
                false
            } else {
                true
            }
        });
        if removed {
            Self::serialize_many(&many).map(|s| (Some(s), many))
        } else {
            Ok((None, many))
        }
    }

    pub fn deserialize_many(s: &str) -> Result<Vec<Self>, String> {
        serde_json::from_str(s).map_err(|e| {
            format!(
                "failed to deserialize user defined nodes: {}",
                display_chain(&e)
            )
        })
    }
}
