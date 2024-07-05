use alloc::string::{String, ToString};
use core::fmt::{self, Debug};

use serde::{Deserialize, Serialize};

// ENDPOINT
// ================================================================================================

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Endpoint {
    protocol: String,
    host: String,
    port: u16,
}

impl Endpoint {
    /// Returns a new instance of [Endpoint] with the specified protocol, host, and port.
    pub const fn new(protocol: String, host: String, port: u16) -> Self {
        Self { protocol, host, port }
    }
}

impl Endpoint {
    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}:{}", self.protocol, self.host, self.port)
    }
}

const MIDEN_NODE_PORT: u16 = 57291;
impl Default for Endpoint {
    fn default() -> Self {
        Self {
            protocol: "http".to_string(),
            host: "localhost".to_string(),
            port: MIDEN_NODE_PORT,
        }
    }
}

impl TryFrom<&str> for Endpoint {
    type Error = String;

    fn try_from(endpoint: &str) -> Result<Self, Self::Error> {
        let protocol_separator_index = endpoint.find("://");
        let port_separator_index = endpoint.rfind(':');

        // port separator index might match with the protocol separator, if so that means there was
        // no port defined
        let port_separator_index = if port_separator_index == protocol_separator_index {
            None
        } else {
            port_separator_index
        };

        let (protocol, hostname, port) = match (protocol_separator_index, port_separator_index) {
            (Some(protocol_idx), Some(port_idx)) => {
                let (protocol_and_hostname, port) = endpoint.split_at(port_idx);
                let port = port[1..].parse::<u16>().map_err(|err| err.to_string())?;

                let (protocol, hostname) = protocol_and_hostname.split_at(protocol_idx);
                // skip the separator
                let hostname = &hostname[3..];

                (protocol, hostname, port)
            },
            (Some(protocol_idx), None) => {
                let (protocol, hostname) = endpoint.split_at(protocol_idx);
                // skip the separator
                let hostname = &hostname[3..];

                (protocol, hostname, MIDEN_NODE_PORT)
            },
            (None, Some(port_idx)) => {
                let (hostname, port) = endpoint.split_at(port_idx);
                let port = port[1..].parse::<u16>().map_err(|err| err.to_string())?;

                ("https", hostname, port)
            },
            (None, None) => ("https", endpoint, MIDEN_NODE_PORT),
        };

        Ok(Endpoint {
            protocol: protocol.to_string(),
            host: hostname.to_string(),
            port,
        })
    }
}

// RPC CONFIG
// ================================================================================================

/// Settings for the RPC client
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RpcConfig {
    /// Address of the Miden node to connect to.
    pub endpoint: Endpoint,
    /// Timeout for the rpc api requests
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

const fn default_timeout() -> u64 {
    10000
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            endpoint: Endpoint::default(),
            timeout_ms: 10000,
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::string::ToString;

    use crate::config::{Endpoint, MIDEN_NODE_PORT};

    #[test]
    fn test_endpoint_parsing_with_hostname_only() {
        let endpoint = Endpoint::try_from("some.test.domain").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "some.test.domain".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_ip() {
        let endpoint = Endpoint::try_from("192.168.0.1").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "192.168.0.1".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_port() {
        let endpoint = Endpoint::try_from("some.test.domain:8000").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "some.test.domain".to_string(),
            port: 8000,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_ip_and_port() {
        let endpoint = Endpoint::try_from("192.168.0.1:8000").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "https".to_string(),
            host: "192.168.0.1".to_string(),
            port: 8000,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_protocol() {
        let endpoint = Endpoint::try_from("http://some.test.domain").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "some.test.domain".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_protocol_and_ip() {
        let endpoint = Endpoint::try_from("http://192.168.0.1").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "192.168.0.1".to_string(),
            port: MIDEN_NODE_PORT,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_both_protocol_and_port() {
        let endpoint = Endpoint::try_from("http://some.test.domain:8080").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "some.test.domain".to_string(),
            port: 8080,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_with_ip_and_protocol_and_port() {
        let endpoint = Endpoint::try_from("http://192.168.0.1:8080").unwrap();
        let expected_endpoint = Endpoint {
            protocol: "http".to_string(),
            host: "192.168.0.1".to_string(),
            port: 8080,
        };

        assert_eq!(endpoint, expected_endpoint);
    }

    #[test]
    fn test_endpoint_parsing_should_fail_for_invalid_port() {
        let endpoint = Endpoint::try_from("some.test.domain:8000/hello");
        assert!(endpoint.is_err());
    }
}
