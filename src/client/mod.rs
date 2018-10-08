// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use std::{path::Path, time::Duration};

use reqwest::{
    header::{HeaderMap, HeaderName, CONTENT_TYPE, RANGE, USER_AGENT},
    Client, StatusCode,
};

use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use update_package::UpdatePackage;
use Result;

#[cfg(test)]
pub(crate) mod tests;

pub(crate) struct Api<'a> {
    server: &'a str,
}

#[derive(Debug)]
pub(crate) enum ProbeResponse {
    NoUpdate,
    Update(UpdatePackage),
    ExtraPoll(i64),
}

impl<'a> Api<'a> {
    pub(crate) fn new(server: &'a str) -> Api<'a> {
        Api { server }
    }

    fn client(&self) -> Result<Client> {
        let mut headers = HeaderMap::new();

        headers.insert(USER_AGENT, "updatehub/next".parse()?);
        headers.insert(CONTENT_TYPE, "application/json".parse()?);
        headers.insert(
            HeaderName::from_static("api-content-type"),
            "application/vnd.updatehub-v1+json".parse()?,
        );

        Ok(Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()?)
    }

    pub fn probe(
        &self,
        runtime_settings: &RuntimeSettings,
        firmware: &Metadata,
    ) -> Result<ProbeResponse> {
        let mut response = self
            .client()?
            .post(&format!("{}/upgrades", &self.server))
            .header(
                HeaderName::from_static("api-retries"),
                runtime_settings.retries(),
            )
            .json(firmware)
            .send()?;

        match response.status() {
            StatusCode::NOT_FOUND => Ok(ProbeResponse::NoUpdate),
            StatusCode::OK => {
                if let Some(extra_poll) = response
                    .headers()
                    .get("add-extra-poll")
                    .and_then(|extra_poll| extra_poll.to_str().ok())
                    .and_then(|extra_poll| extra_poll.parse().ok())
                {
                    return Ok(ProbeResponse::ExtraPoll(extra_poll));
                }

                Ok(ProbeResponse::Update(UpdatePackage::parse(
                    &response.text()?,
                )?))
            }
            _ => bail!("Invalid response. Status: {}", response.status()),
        }
    }

    pub fn download_object(
        &self,
        product_uid: &str,
        package_uid: &str,
        download_dir: &Path,
        object: &str,
    ) -> Result<()> {
        use std::fs::{create_dir_all, OpenOptions};

        // FIXME: Discuss the need of packages inside the route
        let mut client = self.client()?.get(&format!(
            "{}/products/{}/packages/{}/objects/{}",
            &self.server, product_uid, package_uid, object
        ));

        let path = download_dir;
        if !&path.exists() {
            debug!("Creating directory to store the downloads.");
            create_dir_all(&path)?;
        }

        let file = path.join(object);
        if file.exists() {
            client = client.header(RANGE, format!("bytes={}-", file.metadata()?.len() - 1));
        }

        let mut file = OpenOptions::new().create(true).append(true).open(&file)?;
        let mut response = client.send()?;
        if response.status().is_success() {
            response.copy_to(&mut file)?;
            return Ok(());
        }

        bail!("Couldn't download the object {}", object)
    }

    pub fn report(
        &self,
        state: &str,
        firmware: &'a Metadata,
        package_uid: &str,
        previous_state: Option<&str>,
        error_message: Option<String>,
    ) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "kebab-case")]
        struct Payload<'a> {
            status: &'a str,
            #[serde(flatten)]
            firmware: &'a Metadata,
            package_uid: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            previous_state: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            error_message: Option<String>,
        }

        let payload = Payload {
            status: state,
            firmware,
            package_uid,
            previous_state,
            error_message,
        };

        self.client()?
            .post(&format!("{}/report", &self.server))
            .json(&payload)
            .send()?;
        Ok(())
    }
}
