//! The ntp settings can be used to specify time servers with which to synchronize the instance's
//! clock.
use bottlerocket_model_derive::model;
use bottlerocket_modeled_types::{Identifier, Url};
use bottlerocket_settings_sdk::{GenerateResult, LinearlyMigrateable, NoMigration, SettingsModel};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::collections::HashMap;
use std::convert::Infallible;

#[model(impl_default = true)]
pub struct NtpSettingsV1 {
    /// Either the old plain URL list or a named per-server map. The untagged
    /// `NtpTimeServers` enum figures out which one was given, so old configs keep
    /// working and new ones get per-server config without a migration.
    time_servers: NtpTimeServers,
    /// Shared chrony flags for the old URL-list format (applied to every server).
    /// Only used by the Legacy list; the Named map carries flags per server.
    options: Vec<String>,
    /// chrony log categories, rendered as a single `log` line
    /// (e.g. `["measurements","statistics"]` -> `log measurements statistics`).
    /// Empty/unset renders no `log` line.
    logging: Vec<String>,
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

pub mod error {
    use snafu::Snafu;

    /// Errors from parsing the `time-servers` value into one of the two accepted
    /// shapes (a URL list or a named server map).
    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display(
            "time-servers must be a list of URLs or a map of named servers, got {kind}"
        ))]
        WrongType { kind: String },

        #[snafu(display("invalid time-servers URL list: {source}"))]
        InvalidUrlList { source: serde_json::Error },

        #[snafu(display("invalid time-servers named map: {source}"))]
        InvalidNamedMap { source: serde_json::Error },
    }
}

/// The "array or hashmap" shape (shape B): `time-servers` may be EITHER the old
/// plain URL list, OR a named map of per-server objects. Old configs keep working
/// and new ones get per-server config without a V1->V2 migration.
///
/// Per review guidance we do the discrimination in `TryFrom` (via
/// `#[serde(try_from)]`) rather than `#[serde(untagged)]`: serde hands us the raw
/// `serde_json::Value`, and we decide the variant, validate the contents, and
/// return a clear error if it is neither shape.
///   - `Legacy` is the original NtpSettingsV1 list of `Url`s (backwards compatible).
///   - `Named` is the NtpSettingsV2 `HashMap<Identifier, NtpTimeServer>`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value", into = "serde_json::Value")]
pub enum NtpTimeServers {
    /// Old format: a plain list of URLs. Matches what NtpSettingsV1 accepts today.
    Legacy(Vec<Url>),
    /// New format: named per-server config, same map type as NtpSettingsV2.
    Named(HashMap<Identifier, NtpTimeServer>),
}

impl TryFrom<serde_json::Value> for NtpTimeServers {
    type Error = error::Error;

    fn try_from(value: serde_json::Value) -> std::result::Result<Self, Self::Error> {
        match value {
            // A JSON array is the old URL list. Reject any bad URL here.
            serde_json::Value::Array(_) => {
                let list = serde_json::from_value(value).context(error::InvalidUrlListSnafu)?;
                Ok(NtpTimeServers::Legacy(list))
            }
            // A JSON object is the new named map. Reject any bad server entry here.
            serde_json::Value::Object(_) => {
                let map = serde_json::from_value(value).context(error::InvalidNamedMapSnafu)?;
                Ok(NtpTimeServers::Named(map))
            }
            // Anything else (string, number, bool, null) is not a valid shape.
            other => error::WrongTypeSnafu {
                kind: json_kind(&other).to_string(),
            }
            .fail(),
        }
    }
}

impl From<NtpTimeServers> for serde_json::Value {
    fn from(servers: NtpTimeServers) -> Self {
        // Both variants are plain serde types, so this only fails on a
        // programming error (e.g. a non-string map key), not on user input.
        match servers {
            NtpTimeServers::Legacy(list) => serde_json::json!(list),
            NtpTimeServers::Named(map) => serde_json::json!(map),
        }
    }
}

/// A human-readable name for a JSON value's type, for error messages.
fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "a list",
        serde_json::Value::Object(_) => "a map",
    }
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
                logging: None,
            }))
        )
    }

    #[test]
    fn test_serde_ntp() {
        // The old plain URL list still parses — now as the Legacy variant.
        let test_json = r#"{"time-servers":["https://example.net","http://www.example.com"]}"#;

        let ntp: NtpSettingsV1 = serde_json::from_str(test_json).unwrap();
        assert_eq!(
            ntp.time_servers.clone().unwrap(),
            NtpTimeServers::Legacy(vec!(
                Url::try_from("https://example.net").unwrap(),
                Url::try_from("http://www.example.com").unwrap(),
            ))
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
    fn test_serde_ntp_v1_named_map() {
        // Shape B end to end at the V1 level: a user can give the new named map
        // plus top-level logging, and it parses through NtpSettingsV1 as the
        // Named variant. This is the new format old V1 could not express.
        let test_json = r#"{"time-servers":{"link-local":{"address":"169.254.169.123","directive":"server","options":["iburst","prefer","minpoll 4","maxpoll 4"]}},"logging":["tracking"]}"#;

        let ntp: NtpSettingsV1 = serde_json::from_str(test_json).unwrap();
        match ntp.time_servers.clone().unwrap() {
            NtpTimeServers::Named(map) => {
                let server = map
                    .get(&Identifier::try_from("link-local").unwrap())
                    .unwrap();
                assert_eq!(server.directive, Some(NtpDirective::Server));
            }
            NtpTimeServers::Legacy(_) => panic!("named map misparsed as Legacy"),
        }
        assert_eq!(ntp.logging.clone().unwrap(), vec!["tracking".to_string()]);

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

    // Shape B: the TryFrom-based enum must pick the right variant from JSON, on
    // the real Url/Identifier types (the datastore crate covers the flat-store
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

    #[test]
    fn test_tryfrom_rejects_wrong_type_with_clear_error() {
        // The whole point of TryFrom over untagged: a value that is neither a list
        // nor a map fails with a clear, type-specific message (not serde's opaque
        // "did not match any variant").
        let err = serde_json::from_str::<NtpTimeServers>(r#""just-a-string""#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("list of URLs or a map of named servers"),
            "expected a clear shape error, got: {msg}"
        );
    }

    #[test]
    fn test_tryfrom_rejects_bad_url_in_list() {
        // A list is the Legacy shape, but a bad URL inside it must still be rejected
        // (TryFrom validates contents, it does not just discriminate the shape).
        let result = serde_json::from_str::<NtpTimeServers>(r#"["not a url with spaces"]"#);
        assert!(result.is_err(), "a malformed URL in the list should be rejected");
    }
}
