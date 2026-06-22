use std::path::Path;

use wrongsv::{
    BaseCarrierId as WrongsvBaseCarrier, ImportConfig as WrongsvConfig, ImportResolutionHint,
    PayloadNetworkId as WrongsvPayloadNetwork, WrongclAdaptResultDocument, WrongclInspection,
    WrongclMissingField, WrongclProfileView, WrongclSupportLevel as WrongsvSupportLevel,
    build_wrongcl_adapt_plan, build_wrongcl_adapt_result, build_wrongcl_inspection,
    import_resolution_hint, load_import_config_path,
};

use crate::error::{ClientError, Result};

pub type CapabilityReport = WrongclInspection;
pub type ProfileSupport = WrongclProfileView;
pub type AdaptedConfig = WrongclAdaptResultDocument;
pub type SupportLevel = WrongsvSupportLevel;
pub type PayloadNetwork = WrongsvPayloadNetwork;
pub type BaseCarrier = WrongsvBaseCarrier;
pub type MissingField = WrongclMissingField;

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilityAssessment {
    payload_networks: Vec<PayloadNetwork>,
    base_carriers: Vec<BaseCarrier>,
    support: SupportLevel,
    reason: String,
    missing_fields: Vec<MissingField>,
    config_adaptable: bool,
}

pub fn inspect_wrongsv_config(path: impl AsRef<Path>) -> Result<CapabilityReport> {
    let path = path.as_ref();
    let cfg = read_wrongsv_config(path)?;
    let resolution = read_source_of_truth(path, &cfg)?;
    Ok(build_wrongcl_inspection(&cfg, &resolution))
}

pub fn adapt_wrongsv_config(
    path: impl AsRef<Path>,
    server_host: impl Into<String>,
    listen_host: impl Into<String>,
    listen_port: u16,
) -> Result<AdaptedConfig> {
    let path = path.as_ref();
    let cfg = read_wrongsv_config(path)?;
    let resolution = read_source_of_truth(path, &cfg)?;
    let server_host = server_host.into();
    let listen_host = listen_host.into();
    let plan = build_wrongcl_adapt_plan(&cfg, &resolution, &server_host, &listen_host, listen_port)
        .map_err(map_import_spec_error)?;
    Ok(build_wrongcl_adapt_result(&plan))
}

fn read_wrongsv_config(path: impl AsRef<Path>) -> Result<WrongsvConfig> {
    load_import_config_path(path).map_err(ClientError::Config)
}

fn read_source_of_truth(
    path: impl AsRef<Path>,
    cfg: &WrongsvConfig,
) -> Result<ImportResolutionHint> {
    let inspection = wrongsv::inspect_server_config_path(path).map_err(|error| {
        ClientError::Config(format!("wrongsv endpoint diagnostics failed: {error}"))
    })?;
    let local_hint = import_resolution_hint(cfg);
    let local_profile = local_hint.active_profile.clone();
    let local_inspection = build_wrongcl_inspection(cfg, &local_hint);
    let local_missing_fields = local_inspection.missing_fields;
    if !local_missing_fields.is_empty() && inspection.active_profile != local_profile {
        return Ok(local_hint);
    }
    Ok(ImportResolutionHint {
        active_profile: inspection.active_profile,
        payload_networks: inspection.payload_networks,
        base_carriers: inspection.base_carriers,
    })
}

#[cfg(test)]
fn report_for(cfg: &WrongsvConfig) -> CapabilityReport {
    let resolution = import_resolution_hint(cfg);
    build_wrongcl_inspection(cfg, &resolution)
}

#[cfg(test)]
fn active_capability(cfg: &WrongsvConfig) -> CapabilityAssessment {
    let hint = import_resolution_hint(cfg);
    let inspection = build_wrongcl_inspection(cfg, &hint);
    capability_assessment_from_inspection(&inspection)
}

#[cfg(test)]
fn capability_assessment_from_inspection(inspection: &WrongclInspection) -> CapabilityAssessment {
    CapabilityAssessment {
        payload_networks: inspection.payload_networks.clone(),
        base_carriers: inspection.base_carriers.clone(),
        support: inspection.active_support,
        reason: inspection.active_reason.clone(),
        missing_fields: inspection.missing_fields.clone(),
        config_adaptable: inspection.config_adaptable,
    }
}

#[cfg(test)]
fn client_config_from_document(
    document: &wrongsv::WrongclClientConfigDocument,
    validate: bool,
) -> Result<crate::config::ClientConfig> {
    let text = serde_json::to_string(document)?;
    let config = crate::config::ClientConfig::from_legacy_document_json(&text)?;
    if validate {
        config.validate()?;
    }
    Ok(config)
}

#[cfg(test)]
fn client_config_for(
    cfg: WrongsvConfig,
    server_host: String,
    listen_host: String,
    listen_port: u16,
) -> Result<crate::config::ClientConfig> {
    let resolution = import_resolution_hint(&cfg);
    let plan = build_wrongcl_adapt_plan(&cfg, &resolution, &server_host, &listen_host, listen_port)
        .map_err(map_import_spec_error)?;
    let document = match plan.strict_config {
        Some(config) => config,
        None => {
            if matches!(
                plan.inspection.active_support,
                WrongsvSupportLevel::Unsupported
            ) {
                return Err(ClientError::UnsupportedProtocol(format!(
                    "wrongsv profile '{}' is recognized but not implemented in wrongcl yet",
                    plan.inspection.active_profile
                )));
            }
            return Err(ClientError::Config(plan.inspection.active_reason));
        }
    };
    client_config_from_document(&document, true)
}

fn map_import_spec_error(error: String) -> ClientError {
    if error.contains("recognized but not implemented in wrongcl yet") {
        ClientError::UnsupportedProtocol(error)
    } else {
        ClientError::Config(error)
    }
}

#[cfg(test)]
fn active_profile(cfg: &WrongsvConfig) -> &'static str {
    wrongsv::active_profile_id(cfg)
}

#[cfg(test)]
mod tests;
