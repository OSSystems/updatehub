// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use Result;

use client::Api;
use firmware::Metadata;
use states::{Idle, Reboot, State, StateChangeImpl, StateMachine, TransitionCallback};
use update_package::UpdatePackage;

#[derive(Debug, PartialEq)]
pub(super) struct Install {
    pub(super) update_package: UpdatePackage,
}

create_state_step!(Install => Idle);
create_state_step!(Install => Reboot);

impl TransitionCallback for State<Install> {
    fn callback_state_name(&self) -> &'static str {
        "install"
    }
}

fn report_state_handler_progress<S, F>(
    state: &State<S>,
    enter_state: &str,
    leave_state: &str,
    package_uid: &str,
    mut handler: F,
) -> Result<()>
where
    State<S>: StateChangeImpl,
    F: FnMut() -> Result<()>,
{
    let api = Api::new(&state.settings.network.server_address);

    api.report(enter_state, &state.firmware, package_uid, None, None)?;
    handler()?;
    api.report(leave_state, &state.firmware, package_uid, None, None)?;

    Ok(())
}

impl StateChangeImpl for State<Install> {
    fn handle(mut self) -> Result<StateMachine> {
        let package_uid = self.state.update_package.package_uid();
        info!("Installing update: {}", &package_uid);

        report_state_handler_progress(
            &self,
            "installing",
            "installed",
            &package_uid,
            || -> Result<()> {
                // FIXME: Check if A/B install
                // FIXME: Check InstallIfDifferent

                // Ensure we do a probe as soon as possible so full update
                // cycle can be finished.
                self.runtime_settings.force_poll()?;

                // Avoid installing same package twice.
                self.runtime_settings
                    .set_applied_package_uid(&package_uid)?;

                Ok(())
            },
        )?;

        info!("Update installed successfully");
        Ok(StateMachine::Reboot(self.into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use firmware::{
        tests::{create_fake_metadata, FakeDevice},
        Metadata,
    };
    use runtime_settings::RuntimeSettings;
    use settings::Settings;
    use std::fs;
    use tempfile::NamedTempFile;
    use update_package::tests::get_update_package;

    fn fake_install_state() -> State<Install> {
        let tmpfile = NamedTempFile::new().unwrap();
        let tmpfile = tmpfile.path();
        fs::remove_file(&tmpfile).unwrap();

        State {
            settings: Settings::default(),
            runtime_settings: RuntimeSettings::new()
                .load(tmpfile.to_str().unwrap())
                .unwrap(),
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
            state: Install {
                update_package: get_update_package(),
            },
        }
    }

    #[test]
    fn has_package_uid_if_succeed() {
        let machine = StateMachine::Install(fake_install_state()).move_to_next_state();

        match machine {
            Ok(StateMachine::Reboot(s)) => assert_eq!(
                s.runtime_settings.applied_package_uid(),
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
            Ok(StateMachine::Reboot(s)) => assert_eq!(s.runtime_settings.is_polling_forced(), true),
            Ok(s) => panic!("Invalid success: {:?}", s),
            Err(e) => panic!("Invalid error: {:?}", e),
        }
    }

    #[test]
    fn install_has_transition_callback_trait() {
        let state = fake_install_state();
        assert_eq!(state.callback_state_name(), "install");
    }
}
