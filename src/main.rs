use std::fs;
use serde_json;

mod config;

fn main() {

    //Load Configuration
    let config_data = fs::read_to_string("Config.json").expect("No Config.json Present!");

    let config_data: config::Configuration = serde_json::from_str(&config_data).expect("Invalid json!");
}
