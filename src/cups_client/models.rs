use ipp::model::PrinterState;

#[derive(Debug)]
pub struct IppPrintQueueState {
    pub queue_name: String,
    pub description: String,
    pub printer_make: String,
    pub state: PrinterState,
    pub job_count: i32,
    pub state_message: String,
    pub state_reason: String,
    pub cups_version: String,
}
