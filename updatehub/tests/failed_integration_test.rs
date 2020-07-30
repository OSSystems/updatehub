// Copyright (C) 2020 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use common::{
    create_mock_server, format_output_client_log, format_output_server, get_output_server,
    remove_carriage_newline_caracters, remove_timestamp, remove_version, remove_whitespaces,
    run_client_log, run_client_probe, CheckReqTest, FakeServer, Polling, Server, Settings,
};

pub mod common;

#[test]
fn wrong_config_invalid_download_dir() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let mut perms = std::fs::metadata(tmp_dir.path()).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(tmp_dir.path(), perms).unwrap();

    let setup = Settings::default();
    let (mut session, setup) = setup.download_dir(tmp_dir.path().to_path_buf()).init_server();
    let _mocks = create_mock_server(FakeServer::HasUpdate(
        setup.firmware.data.product_uid.clone(),
        CheckReqTest::Disable,
    ));
    let output_server_1 = get_output_server(&mut session, Polling::Disable);

    let output_client = run_client_probe(Server::Standard);
    let output_server_2 = get_output_server(&mut session, Polling::Disable);
    let output_log = run_client_log();

    let (output_server_trce, output_server_info) = format_output_server(output_server_1);

    insta::assert_snapshot!(output_server_info, @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    "###);

    insta::assert_snapshot!(output_server_trce, @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(format_output_server(output_server_2).0.trim(), @r###"
    <timestamp> DEBG receiving probe request
    <timestamp> TRCE Received external request: Probe(None)
    <timestamp> DEBG saving runtime settings from "<file>"...
    <timestamp> TRCE moving to PrepareDownload state to process the update package.
    <timestamp> ERRO error state reached: Permission denied (os error 13)
    <timestamp> INFO returning to machine's entry point
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(remove_carriage_newline_caracters(output_client), @r###"
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
fn wrong_config_invalid_file_config() {
    use std::io::Write;

    let setup = Settings::default();
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let file_path = file.path().to_owned();

    write!(
        file,
        r#"[network]
    server_address=https://api.updatehub.io, listen_socket=localhost:8080;
    
    [storage]

    read_only=false, runtime_settings=/tmp/runtime_settings.conf
    
    [polling]

    enabled=polling_enabled, interval="1d";
    
    [update]

    download_dir=/tmp/, supported_install_modes=["copy", "tarball"]
    
    [firmware]

    metadata=/tmp/updatehub/firmware"#
    )
    .unwrap();

    let (mut session, _setup) = setup.config_file(file_path).init_server();
    let output_server = session.exp_eof().unwrap();
    let output_server =
        remove_carriage_newline_caracters(remove_timestamp(remove_version(output_server)));

    insta::assert_snapshot!(output_server, @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    unexpected character found: `/` at line 2 column 26
    "###);
}

#[test]
fn wrong_config_invalid_server_address() {
    /*
    let setup = Settings::default();
    let (mut session, setup) = setup.server_address("http://foo:--".to_string()).init_server();
    let mocks = create_mock_server(FakeServer::NoUpdate);
    let output_server_1 = get_output_server(&mut session, Polling::Disable);

    let output_client = run_client_probe(Server::Standard);
    let output_server_2 = get_output_server(&mut session, Polling::Disable);
    let output_log = run_client_log();

    println!("{:#?}", output_server_2);*/
}

#[test]
fn wrong_config_check_requirements() {
    let setup = Settings::default();

    let (mut session, setup) = setup.timeout(300).init_server();
    let _mocks = create_mock_server(FakeServer::HasUpdate(
        setup.firmware.data.product_uid.clone(),
        CheckReqTest::Enable,
    ));
    let output_server_1 = get_output_server(&mut session, Polling::Disable);

    let output_client = run_client_probe(Server::Standard);
    let output_server_2 = get_output_server(&mut session, Polling::Disable);
    let output_log = run_client_log();

    let (output_server_trce_1, output_server_info_1) = format_output_server(output_server_1);
    let (output_server_trce_2, output_server_info_2) = format_output_server(output_server_2);
    let output_server_info_2 = remove_whitespaces(
        output_server_info_2,
        FakeServer::HasUpdate(setup.firmware.data.product_uid.clone(), CheckReqTest::Enable),
    );

    insta::assert_snapshot!(output_server_info_1, @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    "###);

    insta::assert_snapshot!(output_server_trce_1, @r###"
    <timestamp> INFO starting UpdateHub Agent <version>
    <timestamp> DEBG loading system settings from "<file>"...
    <timestamp> DEBG runtime settings file "<file>" does not exists, using default settings...
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(output_server_trce_2.trim(), @r###"
    <timestamp> DEBG receiving probe request
    <timestamp> TRCE Received external request: Probe(None)
    <timestamp> DEBG saving runtime settings from "<file>"...
    <timestamp> TRCE moving to PrepareDownload state to process the update package.
    <timestamp> ERRO error state reached: 501
    <timestamp> INFO returning to machine's entry point
    <timestamp> DEBG polling is disabled, parking the state machine.
    <timestamp> DEBG staying on Park state.
    "###);

    insta::assert_snapshot!(output_server_info_2.trim(), @r###"
    <timestamp> ERRO error state reached: 501
    <timestamp> INFO returning to machine's entry point
    "###);

    insta::assert_snapshot!(remove_carriage_newline_caracters(output_client), @r###"
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
