use bottlerocket_settings_sdk::{BottlerocketSetting, LinearMigratorExtensionBuilder};
use settings_extension_ntp::NtpSettingsV2;
use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::init();

    // NOTE: only V2 is registered for now. Coexisting V1 + a V1->V2 migration
    // still needs to be added before shipping (upgrades from V1 nodes won't
    // deserialize without it); fresh installs boot fine on V2 alone.
    match LinearMigratorExtensionBuilder::with_name("ntp")
        .with_models(vec![BottlerocketSetting::<NtpSettingsV2>::model()])
        .build()
    {
        Ok(extension) => extension.run(),
        Err(e) => {
            println!("{e}");
            ExitCode::FAILURE
        }
    }
}
