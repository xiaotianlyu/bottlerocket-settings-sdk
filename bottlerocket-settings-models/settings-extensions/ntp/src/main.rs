use bottlerocket_settings_sdk::{BottlerocketSetting, LinearMigratorExtensionBuilder};
use settings_extension_ntp::NtpSettingsV1;
use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::init();

    // Shape B stays on v1: the reworked NtpSettingsV1 accepts either the old URL
    // list or the new named-server map (via the try_from on NtpTimeServers), so
    // there's no V2 to register and no migration to run.
    match LinearMigratorExtensionBuilder::with_name("ntp")
        .with_models(vec![BottlerocketSetting::<NtpSettingsV1>::model()])
        .build()
    {
        Ok(extension) => extension.run(),
        Err(e) => {
            println!("{e}");
            ExitCode::FAILURE
        }
    }
}
