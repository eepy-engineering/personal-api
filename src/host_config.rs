use std::{collections::HashMap, sync::Arc};

use crate::config::Config;

pub struct HandlerConfig {
  pub config: Arc<Config>,
  pub domains: HashMap<String, DomainEntry>,
}

pub struct DomainEntry {
  pub username: String,
}

impl HandlerConfig {
  pub fn new(config: &Arc<Config>) -> Self {
    let domains = config
      .users
      .iter()
      .filter_map(|(username, user_config)| {
        user_config.domain.as_ref().map(|domain| {
          (
            domain.clone(),
            DomainEntry {
              username: username.clone(),
            },
          )
        })
      })
      .collect();

    HandlerConfig {
      config: config.clone(),
      domains,
    }
  }
}
