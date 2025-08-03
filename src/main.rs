use std::sync::OnceLock;

use config::models::Settings;
use backon::{ExponentialBuilder, Retryable};
use convert_case::{Converter, pattern};
use cups_client::models::IppPrintQueueState;
use dashmap::DashMap;
use log::{debug, error, info};
use mqtt_client::{client::MqttClient, models::*};
use snafu::{OptionExt, ResultExt, Snafu};
use url::Url;
use tokio::{sync::Mutex, task::JoinSet, time::sleep};

use crate::cups_client::client::CupsError;

mod cups_client;
mod config;
mod mqtt_client;

static PRINT_QUEUES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub fn get_settings() -> &'static Settings {
    static LOG_FILE_REGEX: OnceLock<Settings> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| config::loading::load_config())
}

pub fn get_mqtt_client() -> &'static MqttClient {
    static LOG_FILE_REGEX: OnceLock<MqttClient> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| mqtt_client::client::MqttClient::new(&get_settings().mqtt))
}

pub fn get_last_published_mqtt_messages() -> &'static DashMap<String, String> {
    static LOG_FILE_REGEX: OnceLock<DashMap<String, String>> = OnceLock::new();
    LOG_FILE_REGEX.get_or_init(|| DashMap::new())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    // As Rust has no native support for .env files,
    // we use the dotenv_flow crate to import to actual ENV vars.
    let dotenv_path = dotenv_flow::dotenv_flow();
    if dotenv_path.is_ok() {
        println!("Loaded dotenv file: {:?}", dotenv_path.unwrap());
    }

    colog::init();

    info!("Starting cups2mqtt v{}", env!("CARGO_PKG_VERSION"));

    let settings = get_settings();
    info!("Running with config: {:#?}", settings);

    let mut set = JoinSet::new();
    set.spawn(print_queue_status_reporting_loop(settings));

    while let Some(_cert) = set.join_next().await {};
}

async fn failure_wait() {
    error!("Too many failues, waiting 30 seconds before retrying.");
    sleep(std::time::Duration::from_secs(30)).await;
}

async fn publish_cups_queue_statuses_and_log_result() -> Result<(), ApplicationError> {
    let settings = get_settings();
    let url = cups_client::client::build_cups_url(&settings.cups, None).with_whatever_context(|_| "Could not build CUPS URL")?;
    let print_queues_result = cups_client::client::get_print_queues(url, settings.cups.ignore_tls_errors).await;

    match &print_queues_result {
        Ok(print_queues) => {
            debug!("Got print queues: {}", print_queues.len());

            // Update the list of print queues used by the supply levels request loop. Start the loop if not already started.
            if settings.cups.report_supply_levels_schedule.is_some() {
                let print_queues = print_queues.iter().map(|f| f.queue_name.clone()).collect::<Vec<_>>();
                if PRINT_QUEUES.get().is_none() {
                    // Both set the initial list and start the supply levels request loop.
                    PRINT_QUEUES.set(Mutex::new(print_queues)).unwrap();

                    info!("Starting supply levels request loop");
                    tokio::spawn(supply_levels_request_loop(settings));
                } else {
                    // Loop already started, just update the list.
                    let mut queues_guard = PRINT_QUEUES.get().unwrap().lock().await;
                    *queues_guard = print_queues;
                    debug!("Supply levels queue list updated");
                }
            }
        },
        Err(e) => {
            error!("Failed to get print queues (CUPS offline?): {}", e);
        }
    }

    match publish_cups_server_status(&print_queues_result).await {
        Ok(_) => {
            debug!("Published server status");
        },
        Err(e) => {
            error!("Failed to publish server status (MQTT offline?): {}", e);
            return Err(e);
        }
    }

    match print_queues_result {
        Ok(print_queues) => {
            // CUPS online, publish print queues.
            match publish_cups_queue_statuses(&print_queues).await {
                Ok(()) => {
                    debug!("Published queue statuses");
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to publish queue statuses (MQTT offline?): {}", e);
                    Err(e)
                }
            }
        },
        Err(e) => {
            // CUPS offline, publish server status only.
            Err(e).with_whatever_context(|_| "Count not publish queue status due to CUPS error")
        }
    }
}

// ///// //
// Loops //
// ///// //

async fn print_queue_status_reporting_loop(settings: &Settings) {
    loop {
        let cups_print_queues = publish_cups_queue_statuses_and_log_result.retry(ExponentialBuilder::default().with_factor(4.0)).await;
            match cups_print_queues {
            Ok(_) => {
                let duration = settings.polling_schedule.get_duration_till_next_occurrence().unwrap();
                debug!("Next print queue status report run will be in {}", humantime::Duration::from(duration));
                sleep(duration).await;
            },
            Err(_) => {
                error!("Too many queue status reporting failures, waiting 30s before trying again");
                failure_wait().await;
            }
        };
    }
}

async fn supply_levels_request_loop(settings: &Settings) {
    loop {
        debug!("Support levels loop run started");
        for print_queue in PRINT_QUEUES.get().unwrap().lock().await.iter() {
            debug!("Querying support levels for queue {print_queue}");
            let print_queue_uri = cups_client::client::build_cups_url(&settings.cups, Some(print_queue)).unwrap();
            match cups_client::client::report_supply_levels(print_queue_uri, settings.cups.ignore_tls_errors).await {
                Ok(_) => debug!("Successfully queried support levels for queue {print_queue}"),
                Err(error) => debug!("Error while querying support levels for queue: {error}"),
            }
        }

        let sleep_for = settings.cups.report_supply_levels_schedule.as_ref().unwrap().get_duration_till_next_occurrence().unwrap();
        sleep(sleep_for).await;
    }
}

// //////////////////// //
// Print server publish //
// //////////////////// //

async fn publish_cups_server_status(print_queues_result: &Result<Vec<IppPrintQueueState>, CupsError>) -> Result<(), ApplicationError> {
    let settings = get_settings();

    let cups_version = match print_queues_result {
        Ok(print_queues) => print_queues.first().map(|q| q.cups_version.clone()),
        Err(_) => None,
    };

    let topic = format!("{}/{}", settings.mqtt.root_topic, "cups_server");
    let payload = serde_json::to_string(&MqttCupsServerStatus {
        is_reachable: print_queues_result.is_ok(),
        cups_version: cups_version.clone(),
        cups2mqtt_version: env!("CARGO_PKG_VERSION").to_owned(),
    }).with_whatever_context(|_| format!("Could not serialize CUPS server status message for topic {topic}"))?;
    publish(&topic, payload).await?;

    if settings.mqtt.ha.enable_discovery {
        publish_ha_bridge_discovery_topic(&cups_version, "cups_version", "CUPS version").await?;
        publish_ha_bridge_discovery_topic(&cups_version, "cups2mqtt_version", "CUPS2MQTT version").await?;
    }

    Ok(())
}

async fn publish_ha_bridge_discovery_topic(cups_version: &Option<String>, integration_name: &str, sensor_name: &str) -> Result<(), ApplicationError> {
    let settings = get_settings();

    let url = Url::parse(&settings.cups.uri).with_whatever_context(|_| format!("Could not parse CUPS URI {}", settings.cups.uri))?.clone();
    let display_url = format!("{}:{}", url.host_str().with_whatever_context(|| "Failed to get host")?, url.port().unwrap_or(631));

    let topic = format!("{}/sensor/{}_cups_server/{}/config", settings.mqtt.ha.discovery_topic_prefix, settings.mqtt.ha.component_id, integration_name);
    let payload = serde_json::to_string(&HomeAssistantDiscoverySensorPayload {
        name: sensor_name.to_owned(),
        state_topic: format!("{}/cups_server", settings.mqtt.root_topic),
        unique_id: format!("cups_server_{}_{}", integration_name, settings.mqtt.ha.component_id),
        value_template: format!("{{{{ value_json.{} }}}}", integration_name),
        device: HomeAssistantDevice {
            identifiers: vec![format!("{}_cups_server", settings.mqtt.ha.component_id)],
            name: format!("CUPS @ {}", display_url),
            model: "CUPS print server".to_owned(),
            sw_version: cups_version.clone(),
            via_device: None,
        },
    }).with_whatever_context(|_| format!("Could not serialize HA bridge discovery message for topic {topic}"))?;
    Ok(publish(&topic, payload).await?)
}

// /////////////////// //
// Print queue publish //
// /////////////////// //

async fn publish_cups_queue_statuses(print_queues: &Vec<IppPrintQueueState>) -> Result<(), ApplicationError> {
    let settings = get_settings();

    for queue in print_queues {
        let queue_name = queue.queue_name.clone();

        let topic = format!("{}/{}", settings.mqtt.root_topic, queue_name);
        let payload = serde_json::to_string(&MqttCupsPrintQueueStatus::from(queue)).with_whatever_context(|_| format!("Could not serialize CUPS queue status message for topic {topic}"))?;
        publish(&topic, payload).await?;

        if settings.mqtt.ha.enable_discovery {
            publish_ha_sensor_discovery_topic(&queue, "name").await?;
            publish_ha_sensor_discovery_topic(&queue, "description").await?;
            publish_ha_sensor_discovery_topic(&queue, "state").await?;
            publish_ha_sensor_discovery_topic(&queue, "job_count").await?;
            publish_ha_sensor_discovery_topic(&queue, "state_message").await?;
            publish_ha_sensor_discovery_topic(&queue, "state_reason").await?;
        }
    }

    Ok(())
}

async fn publish_ha_sensor_discovery_topic(queue: &IppPrintQueueState, integration_name: &str) -> Result<(), ApplicationError> {
    let settings = get_settings();
    let case_converter = Converter::new().set_pattern(pattern::sentence).set_delim(" ");

    let topic = format!("{}/sensor/{}_{}/{}/config", settings.mqtt.ha.discovery_topic_prefix, settings.mqtt.ha.component_id, queue.queue_name, integration_name);
    let payload = serde_json::to_string(&HomeAssistantDiscoverySensorPayload {
        name: case_converter.convert(integration_name.to_owned()),
        state_topic: format!("{}/{}", settings.mqtt.root_topic, queue.queue_name),
        unique_id: format!("{}_{}_{}", queue.queue_name, integration_name, settings.mqtt.ha.component_id),
        value_template: format!("{{{{ value_json.{} }}}}", integration_name),
        device: HomeAssistantDevice {
            identifiers: vec![format!("{}_{}", settings.mqtt.ha.component_id, queue.queue_name.to_owned())],
            name: queue.description.to_owned(),
            model: queue.printer_make.to_owned(),
            sw_version: None,
            via_device: Some(format!("{}_cups_server", settings.mqtt.ha.component_id)),
        },
    }).with_whatever_context(|_| format!("Could not serialize HA device discovery message for topic {topic}"))?;
    Ok(publish(&topic, payload).await?)
}

// /////// //
// Helpers //
// /////// //

async fn publish(topic: &str, payload: String) -> Result<(), ApplicationError> {
    let last_published = get_last_published_mqtt_messages().get(topic);
    Ok(if last_published.is_none() || !last_published.with_whatever_context(|| "Failed to get last published message")?.eq(&payload) {
        get_last_published_mqtt_messages().insert(topic.to_owned(), payload.clone());
        get_mqtt_client().publish(topic, payload.as_bytes()).await.with_whatever_context(|_| "Could not publish to MQTT")?;
    })
}

// ////// //
// Errors //
// ////// //

#[derive(Debug, Snafu)]
pub enum ApplicationError {
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}
