// Copyright (C) 2019 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use crate::{api, Error, Result};
use attohttpc::{get, post};
use std::path::Path;

#[derive(Clone)]
pub struct Client {
    server_address: String,
}

impl Default for Client {
    fn default() -> Self {
        Client { server_address: "http://localhost:8080".to_string() }
    }
}

impl Client {
    pub fn new(server_address: &str) -> Self {
        Client { server_address: format!("http://{}", server_address), ..Self::default() }
    }

    pub async fn info(&self) -> Result<api::info::Response> {
        let response = get(&format!("{}/info", self.server_address)).send()?;

        match response.status() {
            attohttpc::StatusCode::OK => Ok(response.json()?),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    pub async fn probe(&self, custom: Option<String>) -> Result<api::probe::Response> {
        let response = match custom {
            Some(custom_server) => post(&format!("{}/probe", self.server_address))
                .json(&api::probe::Request { custom_server })?
                .send()?,
            None => post(&format!("{}/probe", self.server_address)).send()?,
        };

        match response.status() {
            attohttpc::StatusCode::OK => Ok(response.json()?),
            attohttpc::StatusCode::ACCEPTED => {
                Err(Error::AgentIsBusy(response.json::<api::state::Response>()?))
            }
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    pub async fn local_install(&self, file: &Path) -> Result<api::state::Response> {
        let response = post(&format!("{}/local_install", self.server_address))
            .header(attohttpc::header::CONTENT_TYPE, "text/plain")
            .text(format!("{}", file.display()))
            .send()?;

        match response.status() {
            attohttpc::StatusCode::OK => Ok(response.json()?),
            attohttpc::StatusCode::UNPROCESSABLE_ENTITY => {
                Err(Error::AgentIsBusy(response.json::<api::state::Response>()?))
            }
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    pub async fn remote_install(&self, url: String) -> Result<api::state::Response> {
        let response = post(&format!("{}/remote_install", self.server_address))
            .header(attohttpc::header::CONTENT_TYPE, "text/plain")
            .text(url)
            .send()?;

        match response.status() {
            attohttpc::StatusCode::OK => Ok(response.json()?),
            attohttpc::StatusCode::UNPROCESSABLE_ENTITY => {
                Err(Error::AgentIsBusy(response.json::<api::state::Response>()?))
            }
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    pub async fn abort_download(&self) -> Result<api::abort_download::Response> {
        let response = post(&format!("{}/update/download/abort", self.server_address)).send()?;

        match response.status() {
            attohttpc::StatusCode::OK => Ok(response.json()?),
            attohttpc::StatusCode::BAD_REQUEST => {
                Err(Error::AbortDownloadRefused(response.json::<api::abort_download::Refused>()?))
            }
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }

    pub async fn log(&self) -> Result<Vec<api::log::Entry>> {
        let response = get(&format!("{}/log", self.server_address)).send()?;

        match response.status() {
            attohttpc::StatusCode::OK => Ok(response.json()?),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
}
