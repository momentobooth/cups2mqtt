use ipp::model::PrinterState;
use serde::{Deserialize, Serialize};

use crate::cups_client::models::IppPrintQueueState;

// ////// //
// Status //
// ////// //

#[derive(Debug, Serialize, Deserialize)]
pub struct MqttCupsServerStatus {
    pub is_reachable: bool,
    pub cups_version: Option<String>,
    pub cups2mqtt_version: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct MqttCupsPrintQueueStatus {
    pub name: String,
    pub description: String,
    pub printer_make: String,
    pub state: MqttCupsPrinterState,
    pub job_count: i32,
    pub state_message: String,
    pub state_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MqttCupsPrinterState {
    Idle = 3,
    Processing = 4,
    Stopped = 5,
}

impl From<&IppPrintQueueState> for MqttCupsPrintQueueStatus {
    fn from(status: &IppPrintQueueState) -> Self {
        MqttCupsPrintQueueStatus {
            name: status.queue_name.clone(),
            description: status.description.clone(),
            printer_make: status.printer_make.clone(),
            state: match status.state {
                PrinterState::Idle => MqttCupsPrinterState::Idle,
                PrinterState::Processing => MqttCupsPrinterState::Processing,
                PrinterState::Stopped => MqttCupsPrinterState::Stopped,
            },
            job_count: status.job_count,
            state_message: status.state_message.clone(),
            state_reason: status.state_reason.clone(),
        }
    }
}

// ////////////// //
// Home Assistant //
// ////////////// //

#[derive(Debug, Serialize, Deserialize)]
pub struct HomeAssistantDiscoverySensorPayload {
    pub name: String,
    pub state_topic: String,
    pub unique_id: String,
    pub device: HomeAssistantDevice,
    pub value_template: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HomeAssistantDiscoveryDeviceTriggerPayload {
    pub automation_type: String,
    pub payload: String,
    pub topic: String,
    pub type_: String,
    pub subtype: String,
    pub device: HomeAssistantDevice,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HomeAssistantDevice {
    pub identifiers: Vec<String>,
    pub model: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sw_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via_device: Option<String>,
}
