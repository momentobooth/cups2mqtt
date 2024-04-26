use rumqttc::{tokio_rustls::rustls::ClientConfig, Client, MqttOptions, QoS};
use std::{sync::Arc, thread, time::Duration};
use anyhow::Result;

use crate::config::models::Mqtt;

use super::fun_with_tls::{get_system_certs, NoopServerCertVerifier};

pub struct MqttClient {
    client: Client,
}

impl MqttClient {
    pub fn new(mqtt_settings: &Mqtt) -> Self {
        let mqtt_options = MqttOptions::new(mqtt_settings.client_id.to_owned(), mqtt_settings.host.to_owned(), mqtt_settings.port)
            .set_credentials(mqtt_settings.username.to_owned(), mqtt_settings.password.to_owned())
            .set_transport(match mqtt_settings.secure {
                true => {
                    let config: ClientConfig = match mqtt_settings.ignore_tls_errors {
                        // TLS without certificate verification.
                        true => ClientConfig::builder().dangerous().with_custom_certificate_verifier(Arc::new(NoopServerCertVerifier {})).with_no_client_auth(),
                        // TLS with certificate verification.
                        false => ClientConfig::builder().with_root_certificates(get_system_certs().clone()).with_no_client_auth(),
                    };
                    rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(Arc::new(config)))
                }
                // No TLS.
                false => rumqttc::Transport::tcp(),
            })
            .set_keep_alive(Duration::from_secs(10)).to_owned();

        let (client, mut connection) = Client::new(mqtt_options, 10);

        thread::spawn(move || {
            for (_i, _notification) in connection.iter().enumerate() {}
        });

        Self { client }
    }

    pub fn publish(&self, topic: &str, payload: &[u8]) -> Result<()> {
        Ok(self.client.publish(topic, QoS::AtLeastOnce, false, payload)?)
    }
}
