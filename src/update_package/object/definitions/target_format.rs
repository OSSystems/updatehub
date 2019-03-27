// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

#[derive(PartialEq, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TargetFormat {
    #[serde(rename = "format?")]
    pub format: bool,
    pub format_options: Option<String>,
}

#[test]
fn deserialize() {
    use serde_json::json;

    assert_eq!(
        TargetFormat {
            format: true,
            format_options: Some("-fs ext2".to_string()),
        },
        serde_json::from_value::<TargetFormat>(json!({
            "format?": true,
            "format-options": "-fs ext2"
        }))
        .unwrap()
    );

    assert_eq!(
        TargetFormat {
            format: false,
            format_options: None,
        },
        serde_json::from_value::<TargetFormat>(json!({
            "format?": false,
        }))
        .unwrap()
    );
}
