// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use Result;

use client::Api;
use firmware::installation_set;
use states::{Idle, Install, State, StateChangeImpl, StateMachine};
use std::fs;
use update_package::{ObjectStatus, UpdatePackage};
use walkdir::WalkDir;

#[derive(Debug, PartialEq)]
pub struct Download {
    pub update_package: UpdatePackage,
}

create_state_step!(Download => Idle);
create_state_step!(Download => Install(update_package));

impl StateChangeImpl for State<Download> {
    fn callback_state_name(&self) -> Option<&'static str> {
        Some("download")
    }

    fn handle(self) -> Result<StateMachine> {
        let installation_set = installation_set::inactive()?;

        // Prune left over from previous installations
        for entry in WalkDir::new(&self.settings.update.download_dir)
            .follow_links(true)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_file())
            .filter_map(|e| e.ok())
            .filter(|e| {
                !self
                    .state
                    .update_package
                    .objects(installation_set)
                    .iter()
                    .map(|o| o.sha256sum())
                    .collect::<Vec<_>>()
                    .contains(&e.file_name().to_str().unwrap_or(""))
            }) {
            fs::remove_file(entry.path())?;
        }

        // Prune corrupted files
        for object in self.state.update_package.filter_objects(
            &self.settings,
            installation_set,
            &ObjectStatus::Corrupted,
        ) {
            fs::remove_file(&self.settings.update.download_dir.join(object.sha256sum()))?;
        }

        // Download the missing or incomplete objects
        for object in self
            .state
            .update_package
            .filter_objects(&self.settings, installation_set, &ObjectStatus::Missing)
            .into_iter()
            .chain(self.state.update_package.filter_objects(
                &self.settings,
                installation_set,
                &ObjectStatus::Incomplete,
            )) {
            Api::new(&self.settings, &self.runtime_settings, &self.firmware)
                .download_object(&self.state.update_package.package_uid(), object.sha256sum())?;
        }

        if self
            .state
            .update_package
            .objects(installation_set)
            .iter()
            .all(|o| o.status(&self.settings.update.download_dir).ok() == Some(ObjectStatus::Ready))
        {
            Ok(StateMachine::Install(self.into()))
        } else {
            bail!("Not all objects are ready for use")
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn fake_download_state() -> State<Download> {
        use firmware::{
            tests::{create_fake_installation_set, create_fake_metadata, FakeDevice},
            Metadata,
        };
        use runtime_settings::RuntimeSettings;
        use std::{env, fs::create_dir_all};
        use update_package::tests::{create_fake_settings, get_update_package};

        let settings = create_fake_settings();
        let tmpdir = settings.update.download_dir.clone();
        let _ = create_dir_all(&tmpdir);
        create_fake_installation_set(&tmpdir, 0);
        env::set_var("PATH", format!("{}", &tmpdir.to_string_lossy()));

        State {
            settings,
            runtime_settings: RuntimeSettings::default(),
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
            state: Download {
                update_package: get_update_package(),
            },
        }
    }

    #[test]
    fn skip_download_if_ready() {
        use update_package::tests::create_fake_object;

        let download_state = fake_download_state();
        let tmpdir = download_state.settings.update.download_dir.clone();

        create_fake_object(&download_state.settings);

        let machine = StateMachine::Download(download_state).move_to_next_state();
        assert_state!(machine, Install);

        assert_eq!(
            WalkDir::new(&tmpdir)
                .follow_links(true)
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| e.file_type().is_file())
                .count(),
            1,
            "Number of objects is wrong"
        );
    }

    #[test]
    fn download_objects() {
        use crypto_hash::{hex_digest, Algorithm};
        use mockito::mock;
        use std::fs::File;
        use std::io::Read;

        let sha256sum = "c775e7b757ede630cd0aa1113bd102661ab38829ca52a6422ab782862f268646";
        let download_state = fake_download_state();
        let tmpdir = download_state.settings.update.download_dir.clone();

        // leftover file to ensure it is removed
        let _ = File::create(&tmpdir.join("leftover-file"));

        let mock = mock(
            "GET",
            format!(
                "/products/{}/packages/{}/objects/{}",
                "229ffd7e08721d716163fc81a2dbaf6c90d449f0a3b009b6a2defe8a0b0d7381",
                &download_state.state.update_package.package_uid(),
                &sha256sum
            ).as_str(),
        ).match_header("Content-Type", "application/json")
        .match_header("Api-Content-Type", "application/vnd.updatehub-v1+json")
        .with_status(200)
        .with_body("1234567890")
        .create();

        let machine = StateMachine::Download(download_state).move_to_next_state();
        assert_state!(machine, Install);

        mock.assert();

        assert_eq!(
            WalkDir::new(&tmpdir)
                .follow_links(true)
                .min_depth(1)
                .into_iter()
                .filter_entry(|e| e.file_type().is_file())
                .count(),
            1,
            "Failed to remove the corrupted object"
        );

        let mut object_content = String::new();
        let _ = File::open(&tmpdir.join(&sha256sum))
            .expect("Fail to open the temporary directory.")
            .read_to_string(&mut object_content);

        assert_eq!(
            &hex_digest(Algorithm::SHA256, object_content.as_bytes()),
            &sha256sum,
            "Checksum mismatch"
        );
    }
}
