// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
mod macros;
mod download;
mod idle;
mod install;
mod park;
mod poll;
mod probe;
mod reboot;
mod transition;

use Result;

use self::{
    download::Download, idle::Idle, install::Install, park::Park, poll::Poll, probe::Probe,
    reboot::Reboot,
};

use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;

trait StateChangeImpl {
    fn handle(self) -> Result<StateMachine>;
}

trait TransitionCallback: StateChangeImpl + Into<State<Idle>> {
    fn callback_state_name(&self) -> &'static str;
}

trait ProgressReporter: TransitionCallback {
    fn package_uid(&self) -> String;
    fn report_enter_state_name(&self) -> &'static str;
    fn report_leave_state_name(&self) -> &'static str;
}

#[derive(Debug, PartialEq)]
struct State<S>
where
    State<S>: StateChangeImpl,
{
    settings: Settings,
    runtime_settings: RuntimeSettings,
    firmware: Metadata,
    state: S,
}

#[derive(Debug, PartialEq)]
enum StateMachine {
    Park(State<Park>),
    Idle(State<Idle>),
    Poll(State<Poll>),
    Probe(State<Probe>),
    Download(State<Download>),
    Install(State<Install>),
    Reboot(State<Reboot>),
}

impl<S> State<S>
where
    State<S>: TransitionCallback + ProgressReporter,
{
    fn handle_with_callback_and_report_progress(self) -> Result<StateMachine> {
        use states::transition::{state_change_callback, Transition};

        let transition = state_change_callback(
            &self.settings.firmware.metadata_path,
            self.callback_state_name(),
        )?;

        match transition {
            Transition::Continue => Ok(self.handle_and_report_progress()?),
            Transition::Cancel => Ok(StateMachine::Idle(self.into())),
        }
    }
}

impl<S> State<S>
where
    State<S>: ProgressReporter,
{
    fn handle_and_report_progress(self) -> Result<StateMachine> {
        let server = self.settings.network.server_address.clone();
        let firmware = self.firmware.clone();
        let package_uid = self.package_uid().clone();
        let enter_state = self.report_enter_state_name();
        let leave_state = self.report_leave_state_name();

        let report = |state, previous_state, error_message| {
            ::client::Api::new(&server).report(
                state,
                &firmware,
                &package_uid,
                previous_state,
                error_message,
            )
        };

        report(enter_state, None, None)?;
        self.handle()
            .and_then(|state| {
                report(leave_state, None, None)?;
                Ok(state)
            })
            .or_else(|e| {
                report("error", Some(enter_state), Some(e.to_string()))?;
                Err(e)
            })
    }
}

impl StateMachine {
    fn new(settings: Settings, runtime_settings: RuntimeSettings, firmware: Metadata) -> Self {
        StateMachine::Idle(State {
            settings,
            runtime_settings,
            firmware,
            state: Idle {},
        })
    }

    fn move_to_next_state(self) -> Result<Self> {
        match self {
            StateMachine::Park(s) => Ok(s.handle()?),
            StateMachine::Idle(s) => Ok(s.handle()?),
            StateMachine::Poll(s) => Ok(s.handle()?),
            StateMachine::Probe(s) => Ok(s.handle()?),
            StateMachine::Download(s) => Ok(s.handle_with_callback_and_report_progress()?),
            StateMachine::Install(s) => Ok(s.handle_with_callback_and_report_progress()?),
            StateMachine::Reboot(s) => Ok(s.handle_with_callback_and_report_progress()?),
        }
    }
}

/// Runs the state machine up to completion handling all procing
/// states without extra manual work.
///
/// It supports following states, and transitions, as shown in the
/// below diagram:
///
/// ```text
///           .--------------.
///           |              v
/// Park <- Idle -> Poll -> Probe -> Download -> Install -> Reboot
///           ^      ^        '          '          '
///           '      '        '          '          '
///           '      `--------'          '          '
///           `---------------'          '          '
///           `--------------------------'          '
///           `-------------------------------------'
/// ```
///
/// # Example
/// ```
/// # extern crate failure;
/// # extern crate updatehub;
/// # use failure;
/// # fn run() -> Result<(), failure::Error> {
/// use updatehub;
///
/// let settings = updatehub::Settings::load()?;
/// updatehub::run(settings)?;
/// # Ok(())
/// # }
/// ```
pub fn run(settings: Settings) -> Result<()> {
    let mut runtime_settings = RuntimeSettings::new().load(&settings.storage.runtime_settings)?;
    if !settings.storage.read_only {
        runtime_settings.enable_persistency();
    }

    let firmware = Metadata::new(&settings.firmware.metadata_path)?;
    let mut machine = StateMachine::new(settings, runtime_settings, firmware);

    // Iterate over the state machine.
    loop {
        machine = match machine.move_to_next_state()? {
            StateMachine::Park(_) => {
                debug!("Parking state machine.");
                return Ok(());
            }
            s => s,
        }
    }
}
