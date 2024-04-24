use serde_derive::Deserialize;

// When changing anything here, make sure to add
// #[serde(alias = "ihavenounderscores")]
// where needed, so it can be read from the ENV vars.

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Mqtt {
    host: String,
    port: u16,
    secure: bool,
    #[serde(alias = "ignoretlserrors")]
    ignore_tls_errors: bool,
    username: String,
    password: String,
    #[serde(alias = "clientid")]
    client_id: String,
    #[serde(alias = "roottopic")]
    root_topic: String,
    ha: HomeAssistant,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct HomeAssistant {
    #[serde(alias = "enablediscovery")]
    enable_discovery: bool,
    #[serde(alias = "discoverytopicprefix")]
    discovery_topic_prefix: String,
    #[serde(alias = "componentid")]
    component_id: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Cups {
    uri: String,
    #[serde(alias = "ignoretlserrors")]
    ignore_tls_errors: bool,
    username: String,
    password: String,
    #[serde(alias = "printqueues")]
    print_queues: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    mqtt: Mqtt,
    cups: Cups,
    #[serde(alias = "sentrydsn")]
    sentry_dsn: Option<String>,
}
