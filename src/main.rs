use std::sync::{OnceLock, Mutex};

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

pub fn get_mqtt_client() -> &'static Mutex<MqttClient> {
    static LOG_FILE_REGEX: OnceLock<Mutex<MqttClient>> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| Mutex::new(mqtt_client::client::MqttClient::new(&get_settings().mqtt)))
}

fn main() {
    colog::init();

    loop {
        let cups_print_queues = get_cups_print_queues.retry(&ExponentialBuilder::default().with_factor(4.0)).call();
        match cups_print_queues {
            Ok(_) => {
                debug!("Successfully published CUPS print queues to MQTT.");
                std::thread::sleep(std::time::Duration::from_secs(1));
            },
            Err(e) => {
                error!("Error publishing CUPS print queues to MQTT: {:?}", e);
                failure_wait();
            }
        }
    }
}

fn failure_wait() {
    error!("Too many failues, waiting 30 seconds before retrying.");
    std::thread::sleep(std::time::Duration::from_secs(30));
}

fn get_cups_print_queues() -> Result<()> {
    let settings = get_settings();
    let url = cups_client::client::build_cups_url(&settings.cups, None);
    let result = cups_client::client::get_printers(url, settings.cups.ignore_tls_errors)?;

    let mqtt_client = get_mqtt_client().lock().unwrap();
    Ok(for queue in result {
        let topic = format!("{}/{}", settings.mqtt.root_topic, queue.queue_name);
        let payload = serde_json::to_string(&MqttCupsPrintQueueStatus::from(queue))?;
        println!("{}: {}", topic, payload);
        mqtt_client.publish(&topic, payload.as_bytes())?;
        println!("Published: {}", topic);
    })
}
