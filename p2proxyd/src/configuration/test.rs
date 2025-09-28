use crate::configuration::{P2ProxydSetup, P2proxydTomlConfig};
use crate::proto::SocketAddrGetResult;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

const SIMPLE_CFG: &str = include_str!("../../../assets/config/simple.toml");

fn zero_pad(input: &str) -> String {
    let mut out = input.to_string();
    while out.len() < p2proxy_lib::proto::HEADER_LENGTH {
        out.push('0');
    }
    out
}

#[test]
fn test_simple_config_parsing() {
    let config = P2proxydTomlConfig::parse_toml(SIMPLE_CFG.as_ref()).unwrap();
    assert_eq!(
        config.secret_key_hex.as_deref().unwrap(),
        "8c3981f6f98d0a09f69931549a883d8ce1c37fbf767c28ace12c81ede4713bfc"
    );
    assert_eq!(config.default_route.as_deref(), Some("default"));
    assert!(config.access_log_path.is_none());
    assert!(config.peers.is_none());
    assert_eq!(1, config.server_ports.len());
    let setup = P2ProxydSetup::from_toml(config).unwrap();
    // Lets anyone through
    let pubk = iroh::SecretKey::from_bytes(&[0u8; 32]).public();
    let SocketAddrGetResult::Allowed(sr) = setup.routes.default_route(&pubk) else {
        panic!("Default route should be allowed");
    };
    assert_eq!(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 4501), sr);
    let SocketAddrGetResult::Allowed(sr) = setup.routes.get(&pubk, &zero_pad("default")) else {
        panic!("\"default\" route should be allowed");
    };
    assert_eq!(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 4501), sr);
}

const EXTENSIVE_CFG: &str = include_str!("../../../assets/config/extensive.toml");

#[test]
fn test_extensive_config_parsing() {
    let config = P2proxydTomlConfig::parse_toml(EXTENSIVE_CFG.as_ref()).unwrap();
    assert_eq!(
        config.secret_key_hex.as_deref().unwrap(),
        "690927f498c370cff79be198b1e6b81e3ec12521d1a76753c8aff67a7bb6f549"
    );
    assert_eq!(config.default_route.as_deref(), Some("demo"));
    assert!(config.access_log_path.is_some());
    assert!(config.peers.is_some());
    assert_eq!(2, config.server_ports.len());
    let setup = P2ProxydSetup::from_toml(config).unwrap();

    let allowed_secret = iroh::SecretKey::from_bytes(
        &hex::decode("145f5e72f8c9fd9173a1b22538a2ecdc2f4a4022428e1029fbb08ca6fa0a785b")
            .unwrap()
            .try_into()
            .unwrap(),
    );
    let anyone = iroh::SecretKey::from_bytes(&[0u8; 32]).public();
    let allowed = allowed_secret.public();
    // Lets anyone through
    for pk in [&anyone, &allowed] {
        let SocketAddrGetResult::Allowed(sr) = setup.routes.default_route(pk) else {
            panic!("Default route should be allowed");
        };
        assert_eq!(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4502), sr);
        let SocketAddrGetResult::Allowed(sr) = setup.routes.get(pk, &zero_pad("demo")) else {
            panic!("\"demo\" route should be allowed");
        };
        assert_eq!(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4502), sr);
    }

    // Allowed can see private
    let SocketAddrGetResult::Allowed(sr) = setup.routes.get(&allowed, &zero_pad("private")) else {
        panic!("\"private\" route should be allowed");
    };
    assert_eq!(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 4503), sr);
    // Anyone can't see private
    let SocketAddrGetResult::NotAllowed = setup.routes.get(&anyone, &zero_pad("private")) else {
        panic!("\"private\" route should be disallowed");
    };
    // disallowed writes to access log
}
