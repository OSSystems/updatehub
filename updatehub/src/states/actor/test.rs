// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    client::tests::{create_mock_server, FakeServer},
    firmware::{
        tests::{create_fake_metadata, FakeDevice},
        Metadata,
    },
    runtime_settings::RuntimeSettings,
    settings::Settings,
};
use actix::{Addr, Arbiter, System};
use futures::future::{self, Future};
use pretty_assertions::assert_eq;
use std::fs;

enum Setup {
    HasUpdate,
    NoUpdate,
}

enum Probe {
    Enabled,
    Disabled,
}

#[derive(Default)]
struct FakeMachine {
    step_count: usize,
    step_expect: usize,
}

impl Actor for FakeMachine {
    type Context = Context<Self>;

    // In tests, only one reference to the Actor's Addr is held, and it is held by
    // the stepper, when it stops the system can be shutdown and we can assert the
    // number of steppers received
    fn stopped(&mut self, _: &mut Context<Self>) {
        assert_eq!(self.step_count, self.step_expect);
        System::current().stop();
    }
}

impl Handler<Step> for FakeMachine {
    type Result = MessageResult<Step>;

    fn handle(&mut self, _: Step, _: &mut Context<Self>) -> Self::Result {
        self.step_count += 1;
        if self.step_count >= self.step_expect {
            MessageResult(super::StepTransition::Never)
        } else {
            MessageResult(super::StepTransition::Immediate)
        }
    }
}

fn setup_actor(kind: Setup, probe: Probe) -> (Addr<Machine>, mockito::Mock, Settings, Metadata) {
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();
    let mut settings = Settings::default();
    settings.polling.enabled = match probe {
        Probe::Enabled => true,
        Probe::Disabled => false,
    };
    let runtime_settings = RuntimeSettings::new()
        .load(tmpfile.to_str().unwrap())
        .unwrap();
    let firmware = Metadata::from_path(&create_fake_metadata(match kind {
        Setup::HasUpdate => FakeDevice::HasUpdate,
        Setup::NoUpdate => FakeDevice::NoUpdate,
    }))
    .unwrap();

    let settings_clone = settings.clone();
    let firmware_clone = firmware.clone();
    let mock = create_mock_server(match kind {
        Setup::HasUpdate => FakeServer::HasUpdate,
        Setup::NoUpdate => FakeServer::NoUpdate,
    });

    (
        // We use the actix::Actor::start here instead of the Machine::start in order to not start
        // the stepper and thus have control of how many steps are been sent to the Machine
        actix::Actor::start(Machine::new(
            StateMachine::Idle(State(Idle {})),
            settings,
            runtime_settings,
            firmware,
        )),
        mock,
        settings_clone,
        firmware_clone,
    )
}

#[test]
fn info_request() {
    let system = System::new("test");

    let (addr, _, settings, firmware) = setup_actor(Setup::NoUpdate, Probe::Enabled);
    Arbiter::spawn(
        addr.send(info::Request)
            .map(move |response| {
                assert_eq!(response.state, "idle");
                assert_eq!(response.version, crate::version().to_string());
                assert_eq!(response.config, settings);
                assert_eq!(response.firmware, firmware);
            })
            .then(|_| {
                System::current().stop();
                future::ok(())
            }),
    );

    system.run().unwrap();
}

#[test]
fn step_sequence() {
    let system = System::new("test");

    let (addr, mock, ..) = setup_actor(Setup::NoUpdate, Probe::Enabled);
    Arbiter::spawn(
        addr.send(info::Request)
            .map(move |response| {
                assert_eq!(response.state, "idle");
                addr
            })
            .and_then(|addr| {
                let f1 = addr.send(Step);
                let f2 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "poll"));
                f1.then(|_| f2).then(|_| future::ok(addr))
            })
            .and_then(|addr| {
                let f1 = addr.send(Step);
                let f2 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "probe"));
                f1.then(|_| f2).then(|_| future::ok(addr))
            })
            .and_then(|addr| {
                let f1 = addr.send(Step);
                let f2 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "idle"));
                f1.then(|_| f2).then(|_| future::ok(addr))
            })
            .then(move |_| {
                mock.assert();
                System::current().stop();
                future::ok(())
            }),
    );

    system.run().unwrap();
}

#[test]
fn download_abort() {
    let system = System::new("test");

    let (addr, mock, ..) = setup_actor(Setup::HasUpdate, Probe::Enabled);
    Arbiter::spawn(
        future::ok::<_, failure::Error>(addr)
            .and_then(|addr| {
                let f1 = addr.send(Step);
                let f2 = addr.send(Step);
                let f3 = addr.send(Step);
                let f4 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "prepare_download"));
                f1.then(|_| f2)
                    .then(|_| f3)
                    .then(|_| f4)
                    .then(|_| future::ok(addr))
            })
            .and_then(|addr| {
                let f1 = addr.send(download_abort::Request);
                let f2 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "idle"));
                f1.then(|_| f2).then(|_| future::ok(addr))
            })
            .then(move |_| {
                mock.assert();
                System::current().stop();
                future::ok(())
            }),
    );

    system.run().unwrap();
}

#[test]
fn trigger_probe() {
    let system = System::new("test");

    let (addr, ..) = setup_actor(Setup::NoUpdate, Probe::Disabled);
    Arbiter::spawn(
        future::ok::<_, failure::Error>(addr)
            .and_then(|addr| {
                let f1 = addr.send(Step);
                let f2 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "park"));
                f1.then(|_| f2).then(|_| future::ok(addr))
            })
            .and_then(|addr| {
                let f1 = addr.send(probe::Request(None));
                let f2 = addr
                    .send(info::Request)
                    .map(|res| assert_eq!(res.state, "probe"));
                f1.then(|_| f2).then(|_| future::ok(addr))
            })
            .then(move |_| {
                System::current().stop();
                future::ok(())
            }),
    );

    system.run().unwrap();
}

#[test]
fn stepper_with_never() {
    let system = System::new("test");

    let mock = actix::Actor::start(FakeMachine {
        step_expect: 15,
        ..FakeMachine::default()
    });
    let mut stepper = super::stepper::Controller::default();

    stepper.start(mock);

    system.run().unwrap();
}
