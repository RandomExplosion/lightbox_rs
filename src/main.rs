use std::fs;
use serde_json;
use chrono::prelude::*;
use std::env;
use log::{info, trace, warn, error};
use pretty_env_logger;

mod conf;

fn main() {
    env::set_var("RUST_LOG", env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()));
    pretty_env_logger::init();

    //while true {
        //Load Configuration as a string
        let config_data = fs::read_to_string("Config.json").expect("No Config.json Present!");
        //Convert to typed struct assemblage for use in the future
        let mut config_data: conf::Configuration = serde_json::from_str(&config_data).expect("Invalid json!");
    //}
}

//Figure out if today is a holiday
fn checkIsHoliday(config: conf::Configuration) -> bool {

    //Get current date and time
    let now = Local::now();

    //Is today a weekend?
    if now.format("%a").to_string() == "Sat" || now.format("%a").to_string() == "Sun" {
        info!("Today is a weekend!\n Using holiday reminders.");
        return true;
    }

    //Is today a public holiday
    if config.public_holidays.iter().any(|day| &day.date == &now.format("%d/%m").to_string()) {
        info!("Today is a public holiday!\nUsing holiday reminders.");
    }
    //Or More readably:
    // for day in config.public_holidays {
    //     if now.format("%d/%m").to_string() == day.date {
    //         info!("Today is {}!\nUsing holiday reminders", day.name);
    //     }
    // }

    for holiday in config.holiday_seasons {
        //Parse start and end dates from strings to datetimes for comparison;
        let start_date = DateTime::parse_from_str(&holiday.start_date, "%d/%m").unwrap();
        let end_date = DateTime::parse_from_str(&holiday.end_date, "%d/%m").unwrap();
        
        //Check if
        if start_date <= now && now <= end_date {
            info!("It's the {0}!\nUsing holiday reminders.", holiday.name);
            return true;
        }
    }

    return false;
}