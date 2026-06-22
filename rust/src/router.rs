use std::net::IpAddr;
use std::str::FromStr;

use ipnet::IpNet;
use serde::{Deserialize, Serialize};

use crate::error::{ClientError, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Direct,
    Proxy(String),
    Reject,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum RuleAction {
    Direct,
    Proxy { name: String },
    Reject,
}

impl RuleAction {
    pub fn to_decision(&self) -> Decision {
        match self {
            RuleAction::Direct => Decision::Direct,
            RuleAction::Proxy { name } => Decision::Proxy(name.clone()),
            RuleAction::Reject => Decision::Reject,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Rule {
    Domain {
        value: String,
        #[serde(flatten)]
        action: RuleAction,
    },
    DomainSuffix {
        value: String,
        #[serde(flatten)]
        action: RuleAction,
    },
    DomainKeyword {
        value: String,
        #[serde(flatten)]
        action: RuleAction,
    },
    IpCidr {
        cidr: String,
        #[serde(flatten)]
        action: RuleAction,
    },
    GeoIp {
        country: String,
        #[serde(flatten)]
        action: RuleAction,
    },
    Match {
        #[serde(flatten)]
        action: RuleAction,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Script {
    pub name: String,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Script {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(ClientError::Config("script name must not be empty".into()));
        }
        for rule in &self.rules {
            if let Rule::IpCidr { cidr, .. } = rule {
                IpNet::from_str(cidr).map_err(|e| {
                    ClientError::Config(format!(
                        "script '{}' has invalid IP-CIDR '{}': {}",
                        self.name, cidr, e
                    ))
                })?;
            }
        }
        Ok(())
    }
}

pub fn evaluate<'a>(rules: &'a [Rule], host: &str) -> Option<&'a RuleAction> {
    evaluate_with_ips(rules, host, &[])
}

pub fn evaluate_with_ips<'a>(
    rules: &'a [Rule],
    host: &str,
    extra_ips: &[IpAddr],
) -> Option<&'a RuleAction> {
    let host = host.trim_end_matches('.');
    let lower = host.to_ascii_lowercase();
    let mut candidate_ips: Vec<IpAddr> = Vec::new();
    if let Ok(ip) = host.parse::<IpAddr>() {
        candidate_ips.push(ip);
    }
    for ip in extra_ips {
        if !candidate_ips.contains(ip) {
            candidate_ips.push(*ip);
        }
    }
    for rule in rules {
        let matched: Option<&RuleAction> = match rule {
            Rule::Domain { value, action } => {
                (lower == value.to_ascii_lowercase()).then_some(action)
            }
            Rule::DomainSuffix { value, action } => {
                let needle = value.trim_start_matches('.').to_ascii_lowercase();
                let suffix_match = lower == needle || lower.ends_with(&format!(".{}", needle));
                suffix_match.then_some(action)
            }
            Rule::DomainKeyword { value, action } => lower
                .contains(&value.to_ascii_lowercase())
                .then_some(action),
            Rule::IpCidr { cidr, action } => IpNet::from_str(cidr).ok().and_then(|net| {
                candidate_ips
                    .iter()
                    .find(|ip| net.contains(*ip))
                    .map(|_| action)
            }),
            Rule::GeoIp { .. } => None,
            Rule::Match { action } => Some(action),
        };
        if let Some(action) = matched {
            return Some(action);
        }
    }
    None
}

pub fn has_ip_cidr_rule(rules: &[Rule]) -> bool {
    rules.iter().any(|r| matches!(r, Rule::IpCidr { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proxy(name: &str) -> RuleAction {
        RuleAction::Proxy {
            name: name.to_string(),
        }
    }

    #[test]
    fn domain_exact_matches_host() {
        let rules = vec![Rule::Domain {
            value: "example.com".into(),
            action: proxy("g"),
        }];
        assert_eq!(evaluate(&rules, "example.com"), Some(&proxy("g")));
        assert_eq!(evaluate(&rules, "www.example.com"), None);
    }

    #[test]
    fn domain_suffix_matches_subdomains_but_not_unrelated() {
        let rules = vec![Rule::DomainSuffix {
            value: "example.com".into(),
            action: proxy("g"),
        }];
        assert_eq!(evaluate(&rules, "example.com"), Some(&proxy("g")));
        assert_eq!(evaluate(&rules, "a.b.example.com"), Some(&proxy("g")));
        assert_eq!(evaluate(&rules, "notexample.com"), None);
    }

    #[test]
    fn domain_keyword_matches_substring_case_insensitive() {
        let rules = vec![Rule::DomainKeyword {
            value: "GoOgLe".into(),
            action: proxy("g"),
        }];
        assert_eq!(evaluate(&rules, "www.google.com"), Some(&proxy("g")));
        assert_eq!(evaluate(&rules, "example.com"), None);
    }

    #[test]
    fn ip_cidr_matches_literal_ipv4() {
        let rules = vec![Rule::IpCidr {
            cidr: "10.0.0.0/8".into(),
            action: RuleAction::Direct,
        }];
        assert_eq!(evaluate(&rules, "10.1.2.3"), Some(&RuleAction::Direct));
        assert_eq!(evaluate(&rules, "192.168.1.1"), None);
        assert_eq!(evaluate(&rules, "example.com"), None);
    }

    #[test]
    fn ip_cidr_matches_ipv6() {
        let rules = vec![Rule::IpCidr {
            cidr: "fe80::/10".into(),
            action: RuleAction::Direct,
        }];
        assert_eq!(evaluate(&rules, "fe80::1"), Some(&RuleAction::Direct));
        assert_eq!(evaluate(&rules, "2001:db8::1"), None);
    }

    #[test]
    fn match_rule_is_terminal_fallback() {
        let rules = vec![
            Rule::Domain {
                value: "skip.example".into(),
                action: RuleAction::Reject,
            },
            Rule::Match { action: proxy("g") },
        ];
        assert_eq!(evaluate(&rules, "anything.com"), Some(&proxy("g")));
        assert_eq!(evaluate(&rules, "skip.example"), Some(&RuleAction::Reject));
    }

    #[test]
    fn geoip_rule_never_matches_in_phase_5() {
        let rules = vec![Rule::GeoIp {
            country: "CN".into(),
            action: RuleAction::Direct,
        }];
        assert_eq!(evaluate(&rules, "example.com"), None);
        assert_eq!(evaluate(&rules, "1.2.3.4"), None);
    }

    #[test]
    fn first_matching_rule_wins() {
        let rules = vec![
            Rule::DomainSuffix {
                value: "example.com".into(),
                action: RuleAction::Reject,
            },
            Rule::Domain {
                value: "www.example.com".into(),
                action: RuleAction::Direct,
            },
        ];
        assert_eq!(
            evaluate(&rules, "www.example.com"),
            Some(&RuleAction::Reject)
        );
    }

    #[test]
    fn script_validate_rejects_bad_cidr() {
        let script = Script {
            name: "user".into(),
            rules: vec![Rule::IpCidr {
                cidr: "not-a-cidr".into(),
                action: RuleAction::Direct,
            }],
        };
        assert!(script.validate().is_err());
    }

    #[test]
    fn script_validate_rejects_empty_name() {
        let script = Script {
            name: "".into(),
            rules: vec![],
        };
        assert!(script.validate().is_err());
    }

    #[test]
    fn json_round_trip_keeps_actions() {
        let rule = Rule::Domain {
            value: "example.com".into(),
            action: proxy("g"),
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\"kind\":\"domain\""));
        assert!(json.contains("\"action\":\"proxy\""));
        assert!(json.contains("\"name\":\"g\""));
        let parsed: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, rule);
    }
}
