use std::sync::OnceLock;

use anyhow::Result;
use config::models::Settings;
use backon::BlockingRetryable;
use backon::ExponentialBuilder;
use log::debug;
use log::error;
use mqtt_client::client::MqttClient;
use mqtt_client::models::MqttCupsPrintQueueStatus;

mod cups_client;
mod config;
mod mqtt_client;

pub fn get_settings() -> &'static Settings {
    static LOG_FILE_REGEX: OnceLock<Settings> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| config::loading::load_config())
}

pub fn get_mqtt_client() -> &'static MqttClient {
    static LOG_FILE_REGEX: OnceLock<MqttClient> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| mqtt_client::client::MqttClient::new(&get_settings().mqtt))
}

fn main() {
    colog::init();

    loop {
        let cups_print_queues = publish_cups_queue_statuses_and_log_result.retry(&ExponentialBuilder::default().with_factor(4.0)).call();
        match cups_print_queues {
            Ok(_) => {
                std::thread::sleep(std::time::Duration::from_secs(1));
            },
            Err(_) => {
                error!("Too many failures, waiting 30s before trying again");
                failure_wait();
            }
        }
    }
}

fn failure_wait() {
    error!("Too many failues, waiting 30 seconds before retrying.");
    std::thread::sleep(std::time::Duration::from_secs(30));
}

fn publish_cups_queue_statuses_and_log_result() -> Result<()> {
    match publish_cups_queue_statuses() {
        Ok(count) => {
            debug!("Published {} queue statuses", count);
            Ok(())
        },
        Err(e) => {
            error!("Failed to publish queue statuses: {}", e);
            Err(e)
        }
    }
}

fn publish_cups_queue_statuses() -> Result<usize> {
    let settings = get_settings();
    let url = cups_client::client::build_cups_url(&settings.cups, None);
    let result = cups_client::client::get_printers(url, settings.cups.ignore_tls_errors)?;

    let queue_count = result.len();
    for queue in result {
        let topic = format!("{}/{}", settings.mqtt.root_topic, queue.queue_name);
        let payload = serde_json::to_string(&MqttCupsPrintQueueStatus::from(queue))?;
        println!("{}: {}", topic, payload);
        get_mqtt_client().publish(&topic, payload.as_bytes())?;
        println!("Published: {}", topic);
    }

    Ok(queue_count)
}
