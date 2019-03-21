// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{
    actor::{download_abort, probe},
    Download, Idle, Poll, State, StateChangeImpl, StateMachine,
};
use crate::client::Api;

use slog::{slog_debug, slog_error, slog_info};
use slog_scope::{debug, error, info};

#[derive(Debug, PartialEq)]
pub(super) struct Probe {
    pub(super) server_address: Option<String>,
}

create_state_step!(Probe => Idle);
create_state_step!(Probe => Poll);

/// Implements the state change for State<Probe>.
impl StateChangeImpl for State<Probe> {
    fn name(&self) -> &'static str {
        "probe"
    }

    fn handle_download_abort(&self) -> download_abort::Response {
        download_abort::Response::InvalidState
    }

    fn handle_trigger_probe(&self) -> probe::Response {
        probe::Response::RequestAccepted(self.name().to_owned())
    }

    fn handle(self) -> Result<StateMachine, failure::Error> {
        use crate::client::ProbeResponse;
        use chrono::{Duration, Utc};
        use std::thread;

        let server_address = self
            .0
            .server_address
            .clone()
            .unwrap_or_else(|| shared_state!().settings.network.server_address.clone());

        let r = loop {
            let probe = Api::new(&server_address)
                .probe(&shared_state!().runtime_settings, &shared_state!().firmware);
            if let Err(e) = probe {
                error!("{}", e);
                shared_state_mut!().runtime_settings.inc_retries();
                thread::sleep(Duration::seconds(1).to_std().unwrap());
            } else {
                shared_state_mut!().runtime_settings.clear_retries();
                break probe?;
            }
        };

        if let ProbeResponse::ExtraPoll(s) = r {
            info!("Delaying the probing as requested by the server.");
            shared_state_mut!()
                .runtime_settings
                .set_polling_extra_interval(Duration::seconds(s))?;
        };

        match r {
            ProbeResponse::NoUpdate => {
                debug!("Moving to Idle state as no update is available.");

                // Store timestamp of last polling
                shared_state_mut!()
                    .runtime_settings
                    .set_last_polling(Utc::now())?;
                Ok(StateMachine::Idle(self.into()))
            }

            ProbeResponse::ExtraPoll(_) => {
                debug!("Moving to Poll state due the extra polling interval.");
                Ok(StateMachine::Poll(self.into()))
            }

            ProbeResponse::Update(u) => {
                // Ensure the package is compatible
                u.compatible_with(&shared_state!().firmware)?;
                // Store timestamp of last polling
                shared_state_mut!()
                    .runtime_settings
                    .set_last_polling(Utc::now())?;

                if Some(u.package_uid()) == shared_state!().runtime_settings.applied_package_uid() {
                    info!(
                        "Not applying the update package. Same package has already been installed."
                    );
                    debug!("Moving to Idle state as this update package is already installed.");
                    Ok(StateMachine::Idle(self.into()))
                } else {
                    debug!("Moving to Download state to process the update package.");
                    Ok(StateMachine::Download(State(Download {
                        update_package: u,
                    })))
                }
            }
        }
    }
}

#[test]
fn update_not_available() {
    use super::*;
    use crate::{
        client::tests::{create_mock_server, FakeServer},
        firmware::tests::{create_fake_metadata, FakeDevice},
    };
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::NoUpdate);

    let settings = Settings::default();
    let runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();
    let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap();
    set_shared_state!(settings, runtime_settings, firmware);

    let machine = StateMachine::Probe(State(Probe {
        server_address: None,
    }))
    .move_to_next_state();

    mock.assert();

    assert_state!(machine, Idle);
}

#[test]
fn update_available() {
    use super::*;
    use crate::{
        client::tests::{create_mock_server, FakeServer},
        firmware::tests::{create_fake_metadata, FakeDevice},
    };
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::HasUpdate);

    let settings = Settings::default();
    let runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();
    let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::HasUpdate)).unwrap();
    set_shared_state!(settings, runtime_settings, firmware);

    let machine = StateMachine::Probe(State(Probe {
        server_address: None,
    }))
    .move_to_next_state();

    mock.assert();

    assert_state!(machine, Download);
}

#[test]
fn invalid_hardware() {
    use super::*;
    use crate::{
        client::tests::{create_mock_server, FakeServer},
        firmware::tests::{create_fake_metadata, FakeDevice},
    };
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::InvalidHardware);

    let settings = Settings::default();
    let runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();
    let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::InvalidHardware)).unwrap();
    set_shared_state!(settings, runtime_settings, firmware);

    let machine = StateMachine::Probe(State(Probe {
        server_address: None,
    }))
    .move_to_next_state();

    mock.assert();

    assert!(machine.is_err(), "Did not catch an incompatible hardware");
}

#[test]
fn extra_poll_interval() {
    use super::*;
    use crate::{
        client::tests::{create_mock_server, FakeServer},
        firmware::tests::{create_fake_metadata, FakeDevice},
    };
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::ExtraPoll);

    let settings = Settings::default();
    let runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();
    let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::ExtraPoll)).unwrap();
    set_shared_state!(settings, runtime_settings, firmware);

    let machine = StateMachine::Probe(State(Probe {
        server_address: None,
    }))
    .move_to_next_state();

    mock.assert();

    assert_state!(machine, Poll);
}

#[test]
fn skip_same_package_uid() {
    use super::*;
    use crate::{
        client::{
            tests::{create_mock_server, FakeServer},
            ProbeResponse,
        },
        firmware::tests::{create_fake_metadata, FakeDevice},
    };
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::HasUpdate).expect(2);

    let mut runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();

    // We first get the package_uid that will be returned so we can
    // use it for the upcoming test.
    //
    // This has been done so we don't need to manually update it every
    // time we change the package payload.
    let probe = Api::new(&Settings::default().network.server_address)
        .probe(
            &RuntimeSettings::default(),
            &Metadata::from_path(&create_fake_metadata(FakeDevice::HasUpdate)).unwrap(),
        )
        .unwrap();

    if let ProbeResponse::Update(u) = probe {
        runtime_settings
            .set_applied_package_uid(&u.package_uid())
            .unwrap();
    }

    let settings = Settings::default();
    let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::HasUpdate)).unwrap();
    set_shared_state!(settings, runtime_settings, firmware);

    let machine = StateMachine::Probe(State(Probe {
        server_address: None,
    }))
    .move_to_next_state();

    mock.assert();

    assert_state!(machine, Idle);
}

#[test]
fn error() {
    use super::*;
    use crate::{
        client::tests::{create_mock_server, FakeServer},
        firmware::tests::{create_fake_metadata, FakeDevice},
    };
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    // The server here waits for the second request which includes the
    // retries to succeed.
    let mock = create_mock_server(FakeServer::ErrorOnce);

    let settings = Settings::default();
    let runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();
    let firmware = Metadata::from_path(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap();
    set_shared_state!(settings, runtime_settings, firmware);

    let machine = StateMachine::Probe(State(Probe {
        server_address: None,
    }))
    .move_to_next_state();

    mock.assert();

    assert_state!(machine, Idle);
}
