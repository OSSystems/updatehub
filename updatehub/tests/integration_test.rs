// Copyright (C) 2020 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use common::{
    create_mock_server, format_output_client_log, format_output_client_probe, format_output_server,
    run_client_log, run_client_probe, run_server, FakeServer, Polling,
};

mod common;

#[test]
fn correct_config_no_update_no_polling() {
    let (output_server, _guard, ..) = run_server(Polling::Disable);
    let output_log = run_client_log();

    insta::assert_snapshot!(format_output_server(output_server), @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> TRCE starting stepper
    <timestamp> TRCE starting State Machine Actor...
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(format_output_client_log(output_log), @r###"
    Ok(
        [
            Entry {
                level: Debug,
                message: "loading system settings from "<file>",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "runtime settings file "<file>",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "starting stepper",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "starting State Machine Actor...",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "polling is disabled, parking the state machine.",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "staying on Park state.",
                time: "<timestamp>",
                data: {},
            },
        ],
    )"###);
}

#[test]
fn correct_config_no_update_polling() {
    let mocks = create_mock_server(FakeServer::NoUpdate);
    let (output_server, _guard, ..) = run_server(Polling::Enable);
    let output_log = run_client_log();

    insta::assert_snapshot!(format_output_server(output_server), @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> TRCE starting stepper
    <timestamp> TRCE starting State Machine Actor...
    <timestamp> DEBG polling is enabled, moving to Poll state.
    <timestamp> INFO forcing to Probe state as we are in time
    <timestamp> DEBG moving to EntryPoint state as no update is available.
    <timestamp> DEBG saving runtime settings from "<file>"...
    <timestamp> DEBG polling is enabled, moving to Poll state.
    <timestamp> DEBG moving to Probe state after delay.
    "###);

    insta::assert_snapshot!(format_output_client_log(output_log), @r###"
    Ok(
        [
            Entry {
                level: Debug,
                message: "moving to Probe state after delay.",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "sleeping stepper thread for: 86399 seconds",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "receiving log request",
                time: "<timestamp>",
                data: {},
            },
        ],
    )"###);

    mocks.iter().for_each(|mock| mock.assert());
}

#[test]
fn correct_config_no_update_polling_probe_api() {
    let mocks = create_mock_server(FakeServer::NoUpdate);
    let (output_server, _guard, ..) = run_server(Polling::Enable);

    mocks.iter().for_each(|mock| mock.assert());

    let output_probe = run_client_probe();
    let output_log = run_client_log();

    insta::assert_snapshot!(format_output_server(output_server), @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> TRCE starting stepper
    <timestamp> TRCE starting State Machine Actor...
    <timestamp> DEBG polling is enabled, moving to Poll state.
    <timestamp> INFO forcing to Probe state as we are in time
    <timestamp> DEBG moving to EntryPoint state as no update is available.
    <timestamp> DEBG saving runtime settings from "<file>"...
    <timestamp> DEBG polling is enabled, moving to Poll state.
    <timestamp> DEBG moving to Probe state after delay.
    "###);
    insta::assert_snapshot!(format_output_client_probe(output_probe), @r###"
    Ok(
        Response {
            update_available: false,
            try_again_in: None,
        },
    )
    "###);
    insta::assert_snapshot!(format_output_client_log(output_log), @r###"
    Ok(
        [
            Entry {
                level: Debug,
                message: "moving to Probe state after delay.",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "sleeping stepper thread for: 86399 seconds",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "receiving log request",
                time: "<timestamp>",
                data: {},
            },
        ],
    )"###);
}

#[test]
fn correct_config_no_update_no_polling_probe_api() {
    let mocks = create_mock_server(FakeServer::NoUpdate);
    let (output_server, _guard, ..) = run_server(Polling::Disable);
    let output_client = run_client_probe();
    let output_log = run_client_log();

    insta::assert_snapshot!(format_output_server(output_server), @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> TRCE starting stepper
    <timestamp> TRCE starting State Machine Actor...
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(format_output_client_probe(output_client), @r###"
    Ok(
        Response {
            update_available: false,
            try_again_in: None,
        },
    )
    "###);

    insta::assert_snapshot!(format_output_client_log(output_log), @r###"
    Ok(
        [
            Entry {
                level: Debug,
                message: "loading system settings from "<file>",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "runtime settings file "<file>",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "starting stepper",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "starting State Machine Actor...",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "polling is disabled, parking the state machine.",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "staying on Park state.",
                time: "<timestamp>",
                data: {},
            },
        ],
    )"###);

    mocks.iter().for_each(|mock| mock.assert());
}
/*
#[test]
fn correct_config_update_no_polling_probe_api() {
    let (output_server, product_uid, _guard) = run_server(Polling::Disable);
    let mocks = create_mock_server(FakeServer::HasUpdate(product_uid));
    let output_client = run_client_probe();
    let output_log = run_client_log();

    insta::assert_snapshot!(format_output_server(output_server), @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> TRCE starting stepper
    <timestamp> TRCE starting State Machine Actor...
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(format_output_client_probe(output_client), @r###"
    Ok(
        Response {
            update_available: true,
            try_again_in: None,
        },
    )
    "###);

    insta::assert_snapshot!(format_output_client_log(output_log), @r###"
    Ok(
        [
            Entry {
                level: Debug,
                message: "loading system settings from "<file>",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "runtime settings file "<file>",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "starting stepper",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Trace,
                message: "starting State Machine Actor...",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "polling is disabled, parking the state machine.",
                time: "<timestamp>",
                data: {},
            },
            Entry {
                level: Debug,
                message: "staying on Park state.",
                time: "<timestamp>",
                data: {},
            },
        ],
    )"###);

    mocks.iter().for_each(|mock| mock.assert());
}*/
