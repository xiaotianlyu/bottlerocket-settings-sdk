//! The ntp settings can be used to specify time servers with which to synchronize the instance's
//! clock.
use bottlerocket_model_derive::model;
use bottlerocket_modeled_types::{Identifier, Url};
use bottlerocket_settings_sdk::{GenerateResult, LinearlyMigrateable, NoMigration, SettingsModel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;

#[model(impl_default = true)]
pub struct NtpSettingsV1 {
    time_servers: Vec<Url>,
    options: Vec<String>,
}

type Result<T> = std::result::Result<T, Infallible>;

impl SettingsModel for NtpSettingsV1 {
    /// the `model` macro makes every field of the `NtpSettingsV1` struct an `Option`, so we can use
    /// the type as its own `PartialKind`.
    type PartialKind = Self;
    type ErrorKind = Infallible;

    fn get_version() -> &'static str {
        "v1"
    }

    fn set(_current_value: Option<Self>, _target: Self) -> Result<()> {
        // Anything that parses as a list of URLs is ok
        Ok(())
    }

    fn generate(
        existing_partial: Option<Self::PartialKind>,
        _dependent_settings: Option<serde_json::Value>,
    ) -> Result<GenerateResult<Self::PartialKind, Self>> {
        Ok(GenerateResult::Complete(
            existing_partial.unwrap_or_default(),
        ))
    }

    fn validate(_value: Self, _validated_settings: Option<serde_json::Value>) -> Result<()> {
        // Anything that parses as a list of URLs is ok
        Ok(())
    }
}

impl LinearlyMigrateable for NtpSettingsV1 {
    type ForwardMigrationTarget = NoMigration;
    type BackwardMigrationTarget = NoMigration;

    fn migrate_forward(&self) -> Result<Self::ForwardMigrationTarget> {
        NoMigration::no_defined_migration()
    }

    fn migrate_backward(&self) -> Result<Self::BackwardMigrationTarget> {
        NoMigration::no_defined_migration()
    }
}

/// Whether a time server is a single endpoint (`server`) or a DNS name that may
/// resolve to several servers (`pool`). Renders as the leading chrony directive.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NtpDirective {
    #[default]
    Server,
    Pool,
}

/// A single named time server. Chrony flags (prefer, minpoll, maxpoll, iburst,
/// ...) go in `options` as plain strings. Options render verbatim, so use chrony
/// syntax without "=" (e.g. "minpoll 4", not "minpoll = 4").
#[model(impl_default = true)]
pub struct NtpTimeServer {
    address: Url,
    directive: NtpDirective,
    options: Vec<String>,
}

/// V2 moves NTP from a flat server list to per-server config, following the
/// `HashMap<Identifier, Item>` pattern used by host-containers and
/// bootstrap-containers.
#[model(impl_default = true)]
pub struct NtpSettingsV2 {
    time_servers: HashMap<Identifier, NtpTimeServer>,
    /// chrony log categories, rendered as a single `log` line
    /// (e.g. `["measurements","statistics"]` -> `log measurements statistics`).
    /// Empty/unset renders no `log` line.
    logging: Vec<String>,
}

impl SettingsModel for NtpSettingsV2 {
    type PartialKind = Self;
    type ErrorKind = Infallible;

    fn get_version() -> &'static str {
        "v2"
    }

    fn set(_current_value: Option<Self>, _target: Self) -> Result<()> {
        // Anything that parses as the per-server map is ok.
        Ok(())
    }

    fn generate(
        existing_partial: Option<Self::PartialKind>,
        _dependent_settings: Option<serde_json::Value>,
    ) -> Result<GenerateResult<Self::PartialKind, Self>> {
        Ok(GenerateResult::Complete(
            existing_partial.unwrap_or_default(),
        ))
    }

    fn validate(_value: Self, _validated_settings: Option<serde_json::Value>) -> Result<()> {
        // Anything that parses as the per-server map is ok.
        Ok(())
    }
}

impl LinearlyMigrateable for NtpSettingsV2 {
    // Migration wiring lands in a follow-up. For now V2 has no forward target
    // and no backward migration, so it builds and tests in isolation.
    type ForwardMigrationTarget = NoMigration;
    type BackwardMigrationTarget = NoMigration;

    fn migrate_forward(&self) -> Result<Self::ForwardMigrationTarget> {
        NoMigration::no_defined_migration()
    }

    fn migrate_backward(&self) -> Result<Self::BackwardMigrationTarget> {
        NoMigration::no_defined_migration()
    }
}

/// PROTOTYPE for the "array or hashmap" shape (shape B). Kept alongside V1/V2 so
/// it can be evaluated without deleting anything; the real rework picks one shape.
///
/// The idea (Arnaldo's suggestion): stay on v1 with NO migration, but let
/// `time-servers` be EITHER the old plain URL list, OR a named map of per-server
/// objects. `#[serde(untagged)]` discriminates the two on the way in, and each
/// variant reuses types that already exist:
///   - `Legacy` is the original NtpSettingsV1 list of `Url`s (backwards compatible).
///   - `Named` is the NtpSettingsV2 `HashMap<Identifier, NtpTimeServer>`.
/// This mirrors the in-repo `KubernetesClusterDnsIp` pattern (single value or a
/// list). The datastore round-trip for this shape is verified separately in the
/// datastore crate; these tests cover the serde layer on the real Url/Identifier
/// types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NtpTimeServers {
    /// Old format: a plain list of URLs. Matches what NtpSettingsV1 accepts today.
    Legacy(Vec<Url>),
    /// New format: named per-server config, same map type as NtpSettingsV2.
    Named(HashMap<Identifier, NtpTimeServer>),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_ntp_settings() {
        assert_eq!(
            NtpSettingsV1::generate(None, None),
            Ok(GenerateResult::Complete(NtpSettingsV1 {
                time_servers: None,
                options: None,
            }))
        )
    }

    #[test]
    fn test_serde_ntp() {
        let test_json = r#"{"time-servers":["https://example.net","http://www.example.com"]}"#;

        let ntp: NtpSettingsV1 = serde_json::from_str(test_json).unwrap();
        assert_eq!(
            ntp.time_servers.clone().unwrap(),
            vec!(
                Url::try_from("https://example.net").unwrap(),
                Url::try_from("http://www.example.com").unwrap(),
            )
        );

        let results = serde_json::to_string(&ntp).unwrap();
        assert_eq!(results, test_json);
    }

    #[test]
    fn test_options_ntp() {
        let test_json = r#"{"time-servers":["https://example.net","http://www.example.com"],"options":["minpoll","1","maxpoll","2"]}"#;

        let ntp: NtpSettingsV1 = serde_json::from_str(test_json).unwrap();
        assert_eq!(
            ntp.options.clone().unwrap(),
            vec!("minpoll", "1", "maxpoll", "2",)
        );

        let results = serde_json::to_string(&ntp).unwrap();
        assert_eq!(results, test_json);
    }

    #[test]
    fn test_generate_ntp_settings_v2() {
        // A fresh V2 with no stored value generates an empty server map.
        assert_eq!(
            NtpSettingsV2::generate(None, None),
            Ok(GenerateResult::Complete(NtpSettingsV2 {
                time_servers: None,
                logging: None,
            }))
        )
    }

    #[test]
    fn test_serde_ntp_v2() {
        // The link-local recovery config: server directive with prefer, minpoll,
        // and maxpoll carried as options (no "=", chrony syntax).
        let test_json = r#"{"time-servers":{"link-local":{"address":"169.254.169.123","directive":"server","options":["iburst","prefer","minpoll 4","maxpoll 4"]}}}"#;

        let ntp: NtpSettingsV2 = serde_json::from_str(test_json).unwrap();
        let server = ntp
            .time_servers
            .clone()
            .unwrap()
            .get(&Identifier::try_from("link-local").unwrap())
            .unwrap()
            .clone();

        assert_eq!(server.address, Some(Url::try_from("169.254.169.123").unwrap()));
        assert_eq!(server.directive, Some(NtpDirective::Server));
        assert_eq!(
            server.options,
            Some(vec![
                "iburst".to_string(),
                "prefer".to_string(),
                "minpoll 4".to_string(),
                "maxpoll 4".to_string(),
            ])
        );

        let results = serde_json::to_string(&ntp).unwrap();
        assert_eq!(results, test_json);
    }

    #[test]
    fn test_serde_ntp_v2_logging() {
        // logging is a top-level list of chrony log categories, parsed
        // independently of the per-server map.
        let test_json = r#"{"logging":["measurements","statistics","tracking"]}"#;

        let ntp: NtpSettingsV2 = serde_json::from_str(test_json).unwrap();
        assert_eq!(
            ntp.logging.clone().unwrap(),
            vec![
                "measurements".to_string(),
                "statistics".to_string(),
                "tracking".to_string(),
            ]
        );

        let results = serde_json::to_string(&ntp).unwrap();
        assert_eq!(results, test_json);
    }

    // Shape B prototype: the untagged enum must pick the right variant from JSON,
    // on the real Url/Identifier types (the datastore crate covers the flat-store
    // round-trip separately, with String stand-ins).

    #[test]
    fn test_untagged_parses_legacy_list() {
        // A plain URL list must parse as the Legacy variant (backwards compatible).
        let test_json = r#"["https://time.aws.com","https://time2.aws.com"]"#;
        let parsed: NtpTimeServers = serde_json::from_str(test_json).unwrap();
        assert_eq!(
            parsed,
            NtpTimeServers::Legacy(vec![
                Url::try_from("https://time.aws.com").unwrap(),
                Url::try_from("https://time2.aws.com").unwrap(),
            ])
        );
    }

    #[test]
    fn test_untagged_parses_named_map() {
        // A named map must parse as the Named variant, carrying per-server config.
        let test_json = r#"{"link-local":{"address":"169.254.169.123","directive":"server","options":["iburst","prefer","minpoll 4","maxpoll 4"]}}"#;
        let parsed: NtpTimeServers = serde_json::from_str(test_json).unwrap();
        match parsed {
            NtpTimeServers::Named(map) => {
                let server = map
                    .get(&Identifier::try_from("link-local").unwrap())
                    .unwrap();
                assert_eq!(server.address, Some(Url::try_from("169.254.169.123").unwrap()));
                assert_eq!(server.directive, Some(NtpDirective::Server));
                assert_eq!(
                    server.options,
                    Some(vec![
                        "iburst".to_string(),
                        "prefer".to_string(),
                        "minpoll 4".to_string(),
                        "maxpoll 4".to_string(),
                    ])
                );
            }
            NtpTimeServers::Legacy(_) => {
                panic!("a named map was misparsed as the Legacy list variant")
            }
        }
    }

    #[test]
    fn test_untagged_no_cross_contamination() {
        // The two formats must not bleed into each other: a list is never Named,
        // and a map is never Legacy.
        let list: NtpTimeServers =
            serde_json::from_str(r#"["https://time.aws.com"]"#).unwrap();
        assert!(matches!(list, NtpTimeServers::Legacy(_)));

        let map: NtpTimeServers = serde_json::from_str(
            r#"{"amazon-pool":{"address":"time.aws.com","directive":"pool","options":["iburst"]}}"#,
        )
        .unwrap();
        assert!(matches!(map, NtpTimeServers::Named(_)));
    }
}
