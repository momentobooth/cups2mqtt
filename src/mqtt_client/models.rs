use ipp::model::PrinterState;
use serde::{Deserialize, Serialize};

use crate::cups_client::models::IppPrinterState;

// ////// //
// Status //
// ////// //

#[derive(Debug, Serialize, Deserialize)]
pub struct MqttCupsPrintQueueStatus {
    pub name: String,
    pub description: String,
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

impl From<IppPrinterState> for MqttCupsPrintQueueStatus {
    fn from(status: IppPrinterState) -> Self {
        MqttCupsPrintQueueStatus {
            name: status.queue_name,
            description: status.description,
            state: match status.state {
                PrinterState::Idle => MqttCupsPrinterState::Idle,
                PrinterState::Processing => MqttCupsPrinterState::Processing,
                PrinterState::Stopped => MqttCupsPrinterState::Stopped,
            },
            job_count: status.job_count,
            state_message: status.state_message,
            state_reason: status.state_reason,
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
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub sw_version: String,
}
