//! Remote-address matching for rules. A rule's `remote_ip` is either an exact address
//! or a CIDR block, so this is the only place that needs prefix arithmetic.

use std::net::IpAddr;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IpMatcherError {
    #[error("'{0}' is not an IP address or CIDR block")]
    Malformed(String),
    #[error("/{bits} is not a valid prefix length for {family}")]
    PrefixLength { bits: u32, family: &'static str },
}

/// A parsed `remote_ip` criterion. Addresses of the other family never match: a rule
/// written for IPv4 must not silently apply to IPv6 traffic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IpMatcher {
    Exact(IpAddr),
    Prefix {
        /// Host bits already cleared at parse time, so `contains` is a plain compare.
        network: IpAddr,
        bits: u8,
    },
}

impl IpMatcher {
    pub fn parse(value: &str) -> Result<Self, IpMatcherError> {
        let value = value.trim();
        let Some((address, prefix)) = value.split_once('/') else {
            return value
                .parse::<IpAddr>()
                .map(IpMatcher::Exact)
                .map_err(|_| IpMatcherError::Malformed(value.to_owned()));
        };

        let address: IpAddr = address
            .parse()
            .map_err(|_| IpMatcherError::Malformed(value.to_owned()))?;
        let bits: u32 = prefix
            .parse()
            .map_err(|_| IpMatcherError::Malformed(value.to_owned()))?;
        let width = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if bits > width {
            return Err(IpMatcherError::PrefixLength {
                bits,
                family: if width == 32 { "IPv4" } else { "IPv6" },
            });
        }
        // A full-width prefix is just the address; keeping one representation means the
        // hot path has one fewer case to consider.
        if bits == width {
            return Ok(IpMatcher::Exact(address));
        }
        Ok(IpMatcher::Prefix {
            network: mask(address, bits as u8),
            bits: bits as u8,
        })
    }

    pub fn contains(&self, address: IpAddr) -> bool {
        match self {
            IpMatcher::Exact(expected) => *expected == address,
            IpMatcher::Prefix { network, bits } => {
                same_family(*network, address) && mask(address, *bits) == *network
            }
        }
    }
}

fn same_family(left: IpAddr, right: IpAddr) -> bool {
    left.is_ipv4() == right.is_ipv4()
}

/// Clears every bit below the prefix. Operates on the raw octets so v4 and v6 share
/// one implementation.
fn mask(address: IpAddr, bits: u8) -> IpAddr {
    match address {
        IpAddr::V4(address) => {
            let mut octets = address.octets();
            mask_octets(&mut octets, bits);
            IpAddr::V4(octets.into())
        }
        IpAddr::V6(address) => {
            let mut octets = address.octets();
            mask_octets(&mut octets, bits);
            IpAddr::V6(octets.into())
        }
    }
}

fn mask_octets(octets: &mut [u8], bits: u8) {
    let whole = usize::from(bits / 8);
    let remainder = bits % 8;
    for (index, octet) in octets.iter_mut().enumerate() {
        if index < whole {
            continue;
        }
        if index == whole && remainder > 0 {
            *octet &= !0u8 << (8 - remainder);
        } else {
            *octet = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(value: &str) -> IpAddr {
        value.parse().expect("test address should parse")
    }

    #[test]
    fn matches_an_exact_address() {
        let matcher = IpMatcher::parse("1.2.3.4").expect("valid");
        assert!(matcher.contains(ip("1.2.3.4")));
        assert!(!matcher.contains(ip("1.2.3.5")));
    }

    #[test]
    fn matches_an_ipv4_prefix() {
        let matcher = IpMatcher::parse("10.0.0.0/8").expect("valid");
        assert!(matcher.contains(ip("10.255.1.1")));
        assert!(!matcher.contains(ip("11.0.0.1")));
    }

    #[test]
    fn matches_a_partial_byte_prefix() {
        let matcher = IpMatcher::parse("192.168.1.0/28").expect("valid");
        assert!(matcher.contains(ip("192.168.1.15")));
        assert!(!matcher.contains(ip("192.168.1.16")));
    }

    #[test]
    fn tolerates_host_bits_in_the_written_network() {
        let matcher = IpMatcher::parse("192.168.1.77/24").expect("valid");
        assert!(matcher.contains(ip("192.168.1.1")));
        assert!(!matcher.contains(ip("192.168.2.1")));
    }

    #[test]
    fn a_zero_prefix_matches_the_whole_family_only() {
        let matcher = IpMatcher::parse("0.0.0.0/0").expect("valid");
        assert!(matcher.contains(ip("8.8.8.8")));
        assert!(!matcher.contains(ip("2606:4700::1111")));
    }

    #[test]
    fn matches_an_ipv6_prefix() {
        let matcher = IpMatcher::parse("2606:4700::/32").expect("valid");
        assert!(matcher.contains(ip("2606:4700:4700::1111")));
        assert!(!matcher.contains(ip("2001:4860:4860::8888")));
        assert!(!matcher.contains(ip("1.2.3.4")));
    }

    #[test]
    fn a_full_width_prefix_collapses_to_exact() {
        assert_eq!(
            IpMatcher::parse("1.2.3.4/32").expect("valid"),
            IpMatcher::Exact(ip("1.2.3.4"))
        );
    }

    #[test]
    fn rejects_malformed_and_oversized_prefixes() {
        assert!(matches!(
            IpMatcher::parse("not-an-ip"),
            Err(IpMatcherError::Malformed(_))
        ));
        assert!(matches!(
            IpMatcher::parse("10.0.0.0/x"),
            Err(IpMatcherError::Malformed(_))
        ));
        assert!(matches!(
            IpMatcher::parse("10.0.0.0/33"),
            Err(IpMatcherError::PrefixLength { bits: 33, .. })
        ));
        assert!(matches!(
            IpMatcher::parse("2606:4700::/129"),
            Err(IpMatcherError::PrefixLength { bits: 129, .. })
        ));
    }
}
