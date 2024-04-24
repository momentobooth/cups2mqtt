use chrono::{DateTime, Utc};
use ipp::model::{JobState, PrinterState};

#[derive(Debug)]
pub struct IppPrinterState {
    pub queue_name: String,
    pub description: String,
    pub state: PrinterState,
    pub job_count: i32,
    pub state_message: String,
    pub state_reason: String,
}

#[derive(Debug)]
pub struct PrintJobState {
    pub name: String,
    pub id: i32,
    pub state: JobState,
    pub reason: String,
    pub created: DateTime<Utc>,
}
