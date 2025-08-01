use serde_derive::Deserialize;
use std::time::Duration;

// When changing anything here, make sure to add
// #[serde(alias = "ihavenounderscores")]
// where needed, so it can be read from the ENV vars.

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Mqtt {
    pub host: String,
    pub port: u16,
    pub secure: bool,
    #[serde(alias = "ignoretlserrors")]
    pub ignore_tls_errors: bool,
    pub username: String,
    pub password: String,
    #[serde(alias = "clientid")]
    pub client_id: String,
    #[serde(alias = "roottopic")]
    pub root_topic: String,
    pub ha: HomeAssistant,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct HomeAssistant {
    #[serde(alias = "enablediscovery")]
    pub enable_discovery: bool,
    #[serde(alias = "discoverytopicprefix")]
    pub discovery_topic_prefix: String,
    #[serde(alias = "componentid")]
    pub component_id: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Cups {
    pub uri: String,
    #[serde(alias = "ignoretlserrors")]
    pub ignore_tls_errors: bool,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    #[serde(alias = "pollinginterval", with = "humantime_serde")]
    pub polling_interval: Duration,
    pub mqtt: Mqtt,
    pub cups: Cups,
    #[serde(alias = "sentrydsn")]
    pub sentry_dsn: Option<String>,
}
