// Copyright (C) 2020 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use mockito::{mock, Mock};
use regex::Regex;
use serde_json::json;
use std::{env, path::PathBuf};

pub enum FakeServer {
    NoUpdate,
    HasUpdate(String),
}

pub enum Polling {
    Enable,
    Disable,
}

pub fn run_server(polling: Polling) -> (String, rexpect::session::PtySession, String) {
    let builder = updatehub::tests::TestEnvironment::build();
    let setup = match polling {
        Polling::Enable => builder,
        Polling::Disable => builder.disable_polling(),
    }
    .server_address(mockito::server_url())
    .add_echo_binary("reboot")
    .finish();

    let cmd = format!(
        "{} server -v trace -c {}",
        cargo_bin("updatehub").to_string_lossy(),
        setup.settings.stored_path.to_string_lossy()
    );

    let product_uid = setup.firmware.data.product_uid.clone();

    let mut handle = rexpect::spawn(&cmd, None).expect("fail to spawn server command");
    let output = handle
        .exp_string(match polling {
            Polling::Enable => "TRCE sleeping stepper thread for:",
            Polling::Disable => "TRCE stopping step messages",
        })
        .expect("fail to match the required string");

    (output, handle, product_uid)
}

pub fn run_client_probe() -> String {
    let cmd = format!("{} client probe", cargo_bin("updatehub").to_string_lossy());
    let mut handle = rexpect::spawn(&cmd, None).unwrap();
    handle.exp_eof().expect("fail to match the EOF for client")
}

pub fn run_client_log() -> String {
    let cmd = format!("{} client log", cargo_bin("updatehub").to_string_lossy());
    let mut handle = rexpect::spawn(&cmd, None).unwrap();
    handle.exp_eof().expect("fail to match the EOF for client")
}

pub fn cargo_bin<S: AsRef<str>>(name: S) -> PathBuf {
    let mut target_dir = env::current_exe().expect("fail to get current binary name");

    target_dir.pop();
    if target_dir.ends_with("deps") {
        target_dir.pop();
    }

    target_dir.join(format!("{}{}", name.as_ref(), env::consts::EXE_SUFFIX))
}

pub fn create_mock_server(server: FakeServer) -> Vec<Mock> {
    use mockito::Matcher;

    let json_update = json!({
        "product": "0123456789",
        "version": "1.2",
        "supported-hardware": ["board"],
        "objects":
        [
            [
                {
                    "mode": "test",
                    "filename": "testfile",
                    "target": "/dev/device1",
                    "sha256sum": "c775e7b757ede630cd0aa1113bd102661ab38829ca52a6422ab782862f268646",
                    "size": 10
                }
            ],
            [
                {
                    "mode": "test",
                    "filename": "testfile",
                    "target": "/dev/device2",
                    "sha256sum": "c775e7b757ede630cd0aa1113bd102661ab38829ca52a6422ab782862f268646",
                    "size": 10
                }
            ]
        ]
    });

    let request_body = Matcher::Json(json!({
        "product-uid": "229ffd7e08721d716163fc81a2dbaf6c90d449f0a3b009b6a2defe8a0b0d7381",
        "version": "1.1",
        "hardware": "board",
        "device-identity": {
            "id1":"value1",
            "id2":"value2"
        },
        "device-attributes": {
            "attr1":"attrvalue1",
            "attr2":"attrvalue2"
        }
    }));

    match server {
        FakeServer::NoUpdate => vec![mock("POST", "/upgrades")
        .match_header("Content-Type", "application/json")
        .match_header("Api-Content-Type", "application/vnd.updatehub-v1+json")
        .match_body(request_body)
        .with_status(404)
        .create()],
        FakeServer::HasUpdate(product_id) => vec![mock("POST", "/upgrades")
        .match_header("Content-Type", "application/json")
        .match_header("Api-Content-Type", "application/vnd.updatehub-v1+json")
        .match_body(request_body)
        .with_status(200)
        .with_header("UH-Signature", &openssl::base64::encode_block(b"some_signature"))
        .with_body(&json_update.to_string())
        .create(),
        mock(
            "GET",
            format!(
                "/products/{}/packages/fc8369ff9d3148bcb63914cb6d35fb596129b24dd3052795a8ba0bab4a536cdf/objects/c775e7b757ede630cd0aa1113bd102661ab38829ca52a6422ab782862f268646",
                product_id)
                .as_str(),
        )
            .match_header("Content-Type", "application/json")
            .match_header("Api-Content-Type", "application/vnd.updatehub-v1+json")
            .with_status(200)
            .with_body("1234")
            .create(),
        ]
    }
}

pub fn format_output_server(s: String) -> String {
    let date_re = Regex::new(r"\b(?:Jan|...|Dec) (\d{2}) (\d{2}):(\d{2}):(\d{2}).(\d{3}) ")
        .expect("fail to compile the date regexp");
    let s = date_re.replace_all(&s, "<timestamp> ");

    let version_re = Regex::new(r"Agent .*").expect("fail to compile the version regexp");
    let s = version_re.replace_all(&s, "Agent <version>");

    let tmpfile_re = Regex::new(r#""/tmp/.tmp.*""#).expect("fail to compile the tmpfile regexp");
    let s = tmpfile_re.replace_all(&s, r#""<file>""#);

    let crlf_re = Regex::new(r"\r\n").expect("fail to compile the crlf regexp");
    let s = crlf_re.replace_all(&s, "\n");

    let mut iter = s.lines();
    iter.next_back();
    iter.fold(String::default(), |acc, l| acc + l + "\n").to_string()
}

pub fn format_output_client_probe(s: String) -> String {
    let crlf_re = Regex::new(r"\r\n").expect("fail to compile the crlf regexp");
    crlf_re.replace_all(&s, "\n").to_string()
}

pub fn format_output_client_log(s: String) -> String {
    let date_re =
        Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2}).(\d{9}) (-|\+)(\d{4})")
            .expect("fail to compile the date regexp");
    let s = date_re.replace_all(&s, "<timestamp>");

    let tmpfile_re = Regex::new(r#"\\"/tmp/.tmp.*""#).expect("fail to compile the tmpfile regexp");
    let s = tmpfile_re.replace_all(&s, r#""<file>""#);

    let crlf_re = Regex::new(r"\r\n").expect("fail to compile the crlf regexp");
    let s = crlf_re.replace_all(&s, "\n").to_string();

    s.to_string()
}
