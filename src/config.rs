use crate::protocol::*;
use futures::lock::Mutex;
use language_server::{
    types::{
        ClientCapabilities, ConfigurationItem, ConfigurationParams, Registration,
        RegistrationParams,
    },
    LanguageClient,
};
use log::{error, warn};
use serde::de::DeserializeOwned;
use std::sync::Arc;

#[derive(Debug)]
pub struct ConfigManager {
    client_capabilities: Arc<ClientCapabilities>,
    options: Mutex<Options>,
}

impl ConfigManager {
    pub fn new(client_capabilities: Arc<ClientCapabilities>) -> Self {
        Self {
            client_capabilities,
            options: Mutex::default(),
        }
    }

    pub async fn get(&self) -> Options {
        self.options.lock().await.clone()
    }

    pub async fn register(&self, client: &dyn LanguageClient) {
        if !self.client_capabilities.has_pull_configuration_support()
            && self.client_capabilities.has_push_configuration_support()
        {
            let registration = Registration {
                id: "pull-config".into(),
                method: "workspace/didChangeConfiguration".into(),
                register_options: None,
            };
            let params = RegistrationParams {
                registrations: vec![registration],
            };

            if let Err(why) = client.register_capability(params).await {
                error!(
                    "Failed to register \"workspace/didChangeConfiguration\": {}",
                    why.message
                );
            }
        }
    }

    pub async fn push(&self, options: serde_json::Value) {
        match serde_json::from_value(options) {
            Ok(options) => {
                *self.options.lock().await = options;
            }
            Err(why) => {
                error!("Invalid configuration: {}", why);
            }
        }
    }

    pub async fn pull(&self, client: &dyn LanguageClient) -> bool {
        if self.client_capabilities.has_pull_configuration_support() {
            let latex = self.pull_section("latex", client).await;
            let bibtex = self.pull_section("bibtex", client).await;

            let new_options = Options {
                latex: Some(latex),
                bibtex: Some(bibtex),
            };
            let mut old_options = self.options.lock().await;
            let has_changed = *old_options != new_options;
            *old_options = new_options;
            has_changed
        } else {
            false
        }
    }

    async fn pull_section<T: DeserializeOwned + Default>(
        &self,
        section: &str,
        client: &dyn LanguageClient,
    ) -> T {
        let params = ConfigurationParams {
            items: vec![ConfigurationItem {
                section: Some(section.into()),
                scope_uri: None,
            }],
        };

        match client.configuration(params).await {
            Ok(json) => match serde_json::from_value::<Vec<T>>(json) {
                Ok(config) => config.into_iter().next().unwrap(),
                Err(_) => {
                    warn!("Invalid configuration: {}", section);
                    T::default()
                }
            },
            Err(why) => {
                error!(
                    "Retrieving configuration for {} failed: {}",
                    section, why.message
                );
                T::default()
            }
        }
    }
}
