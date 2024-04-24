pub mod ipp_client;
pub mod config;

fn main() {
    let settings = config::loading::load_config();

    println!("Hello, world!")
}
