use serde::{Deserialize, Serialize};

/// Capture interface as offered to the UI's interface picker.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureInterfaceDto {
    pub id: String,
    pub name: String,
    pub addresses: Vec<String>,
    pub is_loopback: bool,
    pub is_up: bool,
    pub is_wireless: bool,
    pub is_recommended: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatusDto {
    pub running: bool,
    pub interfaces: Vec<CaptureInterfaceDto>,
    pub error: Option<String>,
}

impl CaptureInterfaceDto {
    pub fn from_interface(interface: &crate::capture::CaptureInterface) -> Self {
        Self {
            id: interface.id.clone(),
            name: interface.name.clone(),
            addresses: interface
                .addresses
                .iter()
                .map(|address| address.to_string())
                .collect(),
            is_loopback: interface.is_loopback,
            is_up: interface.is_up,
            is_wireless: interface.is_wireless,
            is_recommended: interface.is_recommended,
        }
    }
}
