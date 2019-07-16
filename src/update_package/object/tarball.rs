// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{definitions, ObjectInstaller, ObjectType};
use crate::utils;
use serde::Deserialize;
use slog::slog_info;
use slog_scope::info;
use std::path::PathBuf;

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Tarball {
    filename: String,
    filesystem: definitions::Filesystem,
    size: u64,
    sha256sum: String,
    #[serde(flatten)]
    target: definitions::TargetType,
    target_path: PathBuf,

    #[serde(default)]
    compressed: bool,
    #[serde(default)]
    required_uncompressed_size: u64,
    #[serde(flatten, default)]
    target_format: definitions::TargetFormat,
    #[serde(default)]
    mount_options: String,
}

impl_object_type!(Tarball);

impl ObjectInstaller for Tarball {
    fn check_requirements(&self) -> Result<(), failure::Error> {
        info!("'tarball' handle checking requirements");

        match self.target {
            definitions::TargetType::Device(_)
            | definitions::TargetType::UBIVolume(_)
            | definitions::TargetType::MTDName(_) => self.target.valid().map(|_| ()),
        }
    }

    fn install(&self, download_dir: PathBuf) -> Result<(), failure::Error> {
        info!("'tarball' handler Install");

        let device = self.target.get_target()?;
        let filesystem = self.filesystem;
        let mount_options = &self.mount_options;
        let format_options = &self.target_format.format_options;
        let source = download_dir.join(self.sha256sum());

        // FIXME: use required_uncompressed_size
        // if we will format, we check the full size
        // else we check the remaning size

        if self.target_format.should_format {
            utils::fs::format(&device, filesystem, format_options)?;
        }

        utils::fs::mount_map(&device, filesystem, mount_options, |path| {
            let dest = path.join(&self.target_path.strip_prefix("/")?);

            compress_tools::uncompress(
                &source,
                &dest,
                utils::fs::find_compress_tarball_kind(&source)?,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use loopdev;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::{
        env, fs,
        io::{Seek, SeekFrom, Write},
        os::unix::fs::MetadataExt,
        path::Path,
        sync::{Arc, Mutex},
    };
    use tempfile;

    const CONTENT_SIZE: usize = 10240;

    lazy_static! {
        static ref SERIALIZE: Arc<Mutex<()>> = Arc::new(Mutex::default());
    }

    fn exec_test_with_tarball<F>(mut f: F) -> Result<(), failure::Error>
    where
        F: FnMut(&mut Tarball),
    {
        // Generate a sparse file for the faked device use
        let mut image = tempfile::NamedTempFile::new()?;
        image.seek(SeekFrom::Start(1024 * 1024 + CONTENT_SIZE as u64))?;
        image.write_all(&[0])?;

        // Setup faked device
        let (loopdev, device) = {
            // Loop device next_free is not thread safe
            let mutex = SERIALIZE.clone();
            let _mutex = mutex.lock().unwrap();
            let loopdev = loopdev::LoopControl::open()?.next_free()?;
            let device = loopdev.path().unwrap();
            loopdev.attach_file(image.path())?;
            (loopdev, device)
        };

        // Format the faked device
        utils::fs::format(&device, definitions::Filesystem::Ext4, &None)?;

        // Generate base copy object
        let mut obj = Tarball {
            filename: "".to_string(),
            filesystem: definitions::Filesystem::Ext4,
            size: CONTENT_SIZE as u64,
            sha256sum: "tree.tar".to_string(),
            target: definitions::TargetType::Device(device.clone()),
            target_path: PathBuf::from("/"),

            compressed: false,
            required_uncompressed_size: CONTENT_SIZE as u64,
            target_format: definitions::TargetFormat::default(),
            mount_options: String::default(),
        };
        f(&mut obj);

        // Setup preinstall structure
        utils::fs::mount_map(&device, definitions::Filesystem::Ext4, &"", |path| {
            fs::create_dir(path.join("existing_dir"))?;
            Ok(())
        })?;

        // Peform Install
        obj.check_requirements()?;
        obj.setup()?;
        obj.install(env::current_dir()?.join("test/fixtures"))?;

        // Validade File
        utils::fs::mount_map(
            &device,
            obj.filesystem,
            &obj.mount_options.clone(),
            |path| {
                let assert_metadata = |p: &Path| -> Result<(), failure::Error> {
                    let metadata = p.metadata()?;
                    assert_eq!(metadata.mode() % 0o1000, 0o664);
                    assert_eq!(metadata.uid(), 1000);
                    assert_eq!(metadata.gid(), 1000);

                    Ok(())
                };
                let dest = path.join(&obj.target_path.strip_prefix("/")?);
                assert_metadata(&dest.join("tree/branch1/leaf"))?;
                assert_metadata(&dest.join("tree/branch2/leaf"))?;

                Ok(())
            },
        )?;

        loopdev.detach()?;

        Ok(())
    }

    #[test]
    #[ignore]
    fn install_over_formated_partion() {
        exec_test_with_tarball(|obj| obj.target_format.should_format = true).unwrap();
    }

    #[test]
    #[ignore]
    fn install_over_unformated_partion() {
        exec_test_with_tarball(|obj| obj.target_path = PathBuf::from("/existing_dir")).unwrap();
    }

    #[test]
    fn deserialize() {
        assert_eq!(
            Tarball {
                filename: "etc/passwd".to_string(),
                filesystem: definitions::Filesystem::Ext4,
                size: 1024,
                sha256sum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
                target: definitions::TargetType::Device(std::path::PathBuf::from("/dev/sda")),
                target_path: PathBuf::from("/"),

                compressed: false,
                required_uncompressed_size: 0,
                target_format: definitions::TargetFormat::default(),
                mount_options: String::default(),
            },
            serde_json::from_value::<Tarball>(json!({
                "filename": "etc/passwd",
                "size": 1024,
                "sha256sum": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "target-type": "device",
                "target": "/dev/sda",
                "filesystem": "ext4",
                "target-path": "/"
            }))
            .unwrap()
        );
    }
}
