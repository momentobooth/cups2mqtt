use config::{Config, Environment};

use super::models::Settings;

pub fn load_config() -> Settings {
    let config = Config::builder()
        .add_source(Environment::default()
            .prefix("C2M")
            .separator("_")
            .prefix_separator("_")
            .try_parsing(true)
            .with_list_parse_key("CUPS.PRINTQUEUES")
            .list_separator(","))
            .set_default("pollingintervalms", "500").unwrap()
            .set_default("mqtt.host", "localhost").unwrap()
            .set_default("mqtt.host", "localhost").unwrap()
            .set_default("mqtt.port", "1883").unwrap()
            .set_default("mqtt.secure", "false").unwrap()
            .set_default("mqtt.ignoretlserrors", "false").unwrap()
            .set_default("mqtt.username", "").unwrap()
            .set_default("mqtt.password", "").unwrap()
            .set_default("mqtt.clientid", "cups2mqtt").unwrap()
            .set_default("mqtt.roottopic", "cups2mqtt").unwrap()
            .set_default("mqtt.ha.enablediscovery", "false").unwrap()
            .set_default("mqtt.ha.discoverytopicprefix", "homeassistant").unwrap()
            .set_default("mqtt.ha.componentid", "cups2mqtt").unwrap()
            .set_default("cups.uri", "https://localhost:631/").unwrap()
            .set_default("cups.ignoretlserrors", "true").unwrap()
            .set_default("cups.username", "").unwrap()
            .set_default("cups.password", "").unwrap()
            .set_default("sentrydsn", "").unwrap()
        .build().unwrap();

    config.try_deserialize().unwrap()
}
