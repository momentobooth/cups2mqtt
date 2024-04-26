use std::sync::OnceLock;

use anyhow::Result;
use config::models::Settings;
use cups_client::models::IppPrinterState;
use backon::BlockingRetryable;
use backon::ExponentialBuilder;
use log::{error, warn, info, debug, trace};

pub mod cups_client;
pub mod config;

pub fn get_settings() -> &'static Settings {
    static LOG_FILE_REGEX: OnceLock<Settings> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| config::loading::load_config())
}

fn main() {
    colog::init();

    loop {
        let cups_print_queues = get_cups_print_queues.retry(&ExponentialBuilder::default().with_factor(4.0)).call();
        if let Ok(cups_print_queues) = cups_print_queues {
            std::thread::sleep(std::time::Duration::from_secs(1));
        } else {
            failure_wait();
        }
    }
}

fn failure_wait() {
    error!("Too many failues, waiting 30 seconds before retrying.");
    std::thread::sleep(std::time::Duration::from_secs(30));
}

fn get_cups_print_queues() -> Result<Vec<IppPrinterState>> {
    let settings = get_settings();
    let url = cups_client::client::build_cups_url(&settings.cups, None);
    let result = cups_client::client::get_printers(url, settings.cups.ignore_tls_errors);

    match result {
        Ok(result) => {
            debug!("CUPS print queues: {:?}", result);
            Ok(result)
        }
        Err(e) => {
            error!("Error getting CUPS print queues: {:?}", e);
            Err(e)
        }
    }
}
