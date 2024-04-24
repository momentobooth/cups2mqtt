use config::{Config, Environment};

use super::models::Settings;

pub fn load_config() -> Settings {
    // As Rust has no native support for .env files,
    // we use the dotenv_flow crate to import to actual ENV vars.
    let dotenv_path = dotenv_flow::dotenv_flow();
    if dotenv_path.is_ok() {
        println!("Loaded dotenv file: {:?}", dotenv_path.unwrap());
    }

    let config = Config::builder()
        .add_source(Environment::default()
            .prefix("C2M")
            .separator("_")
            .prefix_separator("_")
            .try_parsing(true)
            .with_list_parse_key("CUPS.PRINTQUEUES")
            .list_separator(","))
        .build().unwrap();

    config.try_deserialize().unwrap()
}
