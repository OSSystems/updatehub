// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use serde::Deserialize;

#[derive(PartialEq, Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "target-type", content = "target")]
pub enum TargetType {
    Device(String),
    UBIVolume(String),
    MTDName(String),
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn deserialize() {
        assert_eq!(
            TargetType::Device("/dev/sdb".to_string()),
            serde_json::from_value::<TargetType>(json!({
                "target-type": "device",
                "target": "/dev/sdb",
            }))
            .unwrap()
        );
        assert_eq!(
            TargetType::UBIVolume("system1".to_string()),
            serde_json::from_value::<TargetType>(json!({
                "target-type": "ubivolume",
                "target": "system1",
            }))
            .unwrap()
        );
    }
}
