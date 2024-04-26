use std::sync::OnceLock;

use anyhow::Result;
use config::models::Settings;
use backon::BlockingRetryable;
use backon::ExponentialBuilder;
use convert_case::Converter;
use convert_case::Pattern;
use cups_client::models::IppPrinterState;
use log::debug;
use log::error;
use log::info;
use mqtt_client::client::MqttClient;
use mqtt_client::models::HomeAssistantDevice;
use mqtt_client::models::HomeAssistantDiscoverySensorPayload;
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

    info!("Starting cups2mqtt v{}", env!("CARGO_PKG_VERSION"));

    let settings = get_settings();
    info!("Running with config: {:#?}", settings);

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
        let queue_name = queue.queue_name.clone();

        let topic = format!("{}/{}", settings.mqtt.root_topic, queue_name);
        let payload = serde_json::to_string(&MqttCupsPrintQueueStatus::from(&queue))?;
        get_mqtt_client().publish(&topic, payload.as_bytes())?;

        if settings.mqtt.ha.enable_discovery {
            publish_ha_sensor_discovery_topic(&queue, "name".to_owned())?;
            publish_ha_sensor_discovery_topic(&queue, "description".to_owned())?;
            publish_ha_sensor_discovery_topic(&queue, "state".to_owned())?;
            publish_ha_sensor_discovery_topic(&queue, "job_count".to_owned())?;
            publish_ha_sensor_discovery_topic(&queue, "state_message".to_owned())?;
            publish_ha_sensor_discovery_topic(&queue, "state_reason".to_owned())?;
        }
    }

    Ok(queue_count)
}

fn publish_ha_sensor_discovery_topic(queue: &IppPrinterState, integration_name: String) -> Result<()> {
    let settings = get_settings();
    let case_converter = Converter::new().set_pattern(Pattern::Sentence).set_delim(" ");

    let topic = format!("{}/sensor/{}/{}/config", settings.mqtt.ha.discovery_topic_prefix, queue.queue_name, integration_name);
    let payload = serde_json::to_string(&HomeAssistantDiscoverySensorPayload {
        name: case_converter.convert(integration_name.to_owned()),
        state_topic: format!("{}/{}", settings.mqtt.root_topic, queue.queue_name),
        unique_id: format!("{}_{}_{}", queue.queue_name, integration_name, settings.mqtt.ha.component_id),
        value_template: format!("{{{{ value_json.{} }}}}", integration_name),
        device: HomeAssistantDevice {
            identifiers: vec![format!("{}_{}", settings.mqtt.ha.component_id, queue.queue_name.to_owned())],
            name: queue.description.to_owned(),
            model: queue.printer_make.to_owned(),
            sw_version: env!("CARGO_PKG_VERSION").to_owned(),
        },
    })?;
    Ok(get_mqtt_client().publish(&topic, payload.as_bytes())?)
}
