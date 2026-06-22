use crate::error::{ClientError, Result};

use super::TunEnableConfig;

#[allow(dead_code)]
pub struct MacosTunRuntimeHandle {
    interface_name: String,
}

#[allow(dead_code)]
impl MacosTunRuntimeHandle {
    pub fn start(config: TunEnableConfig) -> Result<Self> {
        Err(ClientError::Config(format!(
            "macOS TUN runtime is planned but not implemented yet for interface '{}'. Complete and validate the native host path on a real macOS machine first.",
            config.interface_name
        )))
    }

    pub fn interface_name(&self) -> &str {
        &self.interface_name
    }
}
