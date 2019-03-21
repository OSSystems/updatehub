// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{
    actor::{download_abort, probe},
    Idle, ProgressReporter, Reboot, State, StateChangeImpl, StateMachine, TransitionCallback,
};
use crate::update_package::UpdatePackage;
use slog::slog_info;
use slog_scope::info;

#[derive(Debug, PartialEq)]
pub(super) struct Install {
    pub(super) update_package: UpdatePackage,
}

create_state_step!(Install => Idle);
create_state_step!(Install => Reboot(update_package));

impl TransitionCallback for State<Install> {}

impl ProgressReporter for State<Install> {
    fn package_uid(&self) -> String {
        self.0.update_package.package_uid()
    }

    fn report_enter_state_name(&self) -> &'static str {
        "installing"
    }

    fn report_leave_state_name(&self) -> &'static str {
        "installed"
    }
}

impl StateChangeImpl for State<Install> {
    fn name(&self) -> &'static str {
        "install"
    }

    fn handle_download_abort(&self) -> download_abort::Response {
        download_abort::Response::InvalidState
    }

    fn handle_trigger_probe(&self) -> probe::Response {
        probe::Response::InvalidState(self.name().to_owned())
    }

    fn handle(self) -> Result<StateMachine, failure::Error> {
        let package_uid = self.0.update_package.package_uid();
        info!("Installing update: {}", &package_uid);

        // FIXME: Check if A/B install
        // FIXME: Check InstallIfDifferent

        // Ensure we do a probe as soon as possible so full update
        // cycle can be finished.
        shared_state_mut!().runtime_settings.force_poll()?;

        // Avoid installing same package twice.
        shared_state_mut!()
            .runtime_settings
            .set_applied_package_uid(&package_uid)?;

        info!("Update installed successfully");
        let buffer = crate::logger::buffer();
        buffer.lock().unwrap().stop_logging();
        buffer.lock().unwrap().clear();
        Ok(StateMachine::Reboot(self.into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        firmware::Metadata, runtime_settings::RuntimeSettings, settings::Settings,
        update_package::tests::get_update_package,
    };
    use std::fs;
    use tempfile::NamedTempFile;

    fn fake_install_state() -> State<Install> {
        use crate::firmware::tests::{create_fake_metadata, FakeDevice};

        let tmpfile = NamedTempFile::new().unwrap();
        let tmpfile = tmpfile.path();
        fs::remove_file(&tmpfile).unwrap();

        let settings = Settings::default();
        let runtime_settings = RuntimeSettings::default();
        let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap();
        set_shared_state!(settings, runtime_settings, firmware);

        State(Install {
            update_package: get_update_package(),
        })
    }

    #[test]
    fn has_package_uid_if_succeed() {
        let machine = StateMachine::Install(fake_install_state()).move_to_next_state();

        match machine {
            Ok(StateMachine::Reboot(_)) => assert_eq!(
                shared_state!().runtime_settings.applied_package_uid(),
                Some(get_update_package().package_uid())
            ),
            Ok(s) => panic!("Invalid success: {:?}", s),
            Err(e) => panic!("Invalid error: {:?}", e),
        }
    }

    #[test]
    fn polling_now_if_succeed() {
        let machine = StateMachine::Install(fake_install_state()).move_to_next_state();

        match machine {
            Ok(StateMachine::Reboot(_)) => {
                assert_eq!(shared_state!().runtime_settings.is_polling_forced(), true)
            }
            Ok(s) => panic!("Invalid success: {:?}", s),
            Err(e) => panic!("Invalid error: {:?}", e),
        }
    }

    #[test]
    fn install_has_transition_callback_trait() {
        let state = fake_install_state();
        assert_eq!(state.name(), "install");
    }
}
