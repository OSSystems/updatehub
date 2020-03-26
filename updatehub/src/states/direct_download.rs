// Copyright (C) 2020 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: Apache-2.0

use super::{
    actor::{self, SharedState},
    PrepareLocalInstall, Result, State, StateChangeImpl, StateMachine, TransitionError,
};
use slog_scope::{error, info};
use std::fs;

#[derive(Debug, PartialEq)]
pub(super) struct DirectDownload {
    pub(super) url: String,
}

#[async_trait::async_trait(?Send)]
impl StateChangeImpl for State<DirectDownload> {
    fn name(&self) -> &'static str {
        "direct_download"
    }

    async fn handle(
        self,
        shared_state: &mut SharedState,
    ) -> Result<(StateMachine, actor::StepTransition)> {
        info!("Fetching update package directly from url: {:?}", self.0.url);

        let update_file = shared_state.settings.update.download_dir.join("fetched_pkg");
        let response = attohttpc::get(&self.0.url).send().map_err(|e| {
            error!("Request error: {}", e);
            TransitionError::InvalidRequest
        })?;

        if response.status().is_success() {
            let file = fs::OpenOptions::new().create(true).append(true).open(&update_file)?;
            response.write_to(file)?;
        }

        Ok((
            StateMachine::PrepareLocalInstall(State(PrepareLocalInstall { update_file })),
            actor::StepTransition::Immediate,
        ))
    }
}
