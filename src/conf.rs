use chrono::prelude::*;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::sync::Arc;

const ALLOWED_PINS: [u8; 25] = [
    2, 3, 4, 17, 27, 22, 10, 9, 11, 5, 6, 13, 19, 26, 18, 23, 24, 25, 8, 7, 1, 12, 16, 20, 21,
];

/*
*   This file contains the struct equivalents of the json object in the config file
*   Config is loaded from Config.json at startup and stored in a static Arc<Configuration> for use in threads
*   For more info see ConfigSchema.json
*/

//Lazy static to globally access config
lazy_static! {
    pub static ref CONFIG: Arc<Configuration> = Arc::new(Configuration::load("Config.json"));
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{name:?}'s {t:?} cannot be attached to pin {pin:?} because it is reserved for other uses. Reminders for this user have been disabled.")]
    Pin { t: String, pin: u8, name: String },
}

#[derive(Serialize, Deserialize)]
pub struct HolidaySeason {
    pub name: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Serialize, Deserialize)]
pub struct Holiday {
    pub name: String,
    pub date: String,
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
    pub label: String,
    pub vocal_reminder: Option<String>,
    pub light_on: String,
    pub grace_period: u32,
}

#[derive(Serialize, Deserialize)]
pub struct PinConfig {
    pub button: u8,
    pub led: u8,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub reminders: Vec<Reminder>,
    pub reminders_h: Vec<Reminder>,
    pub pin_config: PinConfig,
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub tts_lan: String,
    pub howler_interval: u32,
    pub snooze_pin: u8,
    pub public_holidays: Vec<Holiday>,
    pub holiday_seasons: Vec<HolidaySeason>,
    pub users: Vec<User>,
}

impl Configuration {
    fn load<P: AsRef<std::path::Path>>(filepath: P) -> Self {
        info!("HOST: Loading configuration file.");
        //Load Configuration as a string
        let config_data = fs::read_to_string(filepath).expect("No Config.json present!");
        //Convert to typed struct assemblage
        let mut config_data: Configuration =
            serde_json::from_str(&config_data).expect("Invalid formatting in Config.json!");
        //Validate configuration and correct if necessary
        config_data.validate_config();
        info!("HOST: Configuration Loaded Successfully!");
        config_data
    }

    //Checks that configuration is valid
    pub fn validate_config(&mut self) {
        //TODO: Check tts language

        //Init Vector to store Used pins
        let mut used_pins: Vec<u8> = Vec::new();

        //Resolve the dates, adding year information
        let yearnow = Local::now().year();
        for holiday in &mut self.public_holidays {
            holiday.date = format!("{}/{}", &holiday.date, &yearnow.to_string());
        }

        for holidayseason in &mut self.holiday_seasons {
            holidayseason.start_date =
                format!("{}/{}", &holidayseason.start_date, &yearnow.to_string());
            holidayseason.end_date =
                format!("{}/{}", &holidayseason.end_date, &yearnow.to_string());
        }

        //Regex for checking dates
        let re = Regex::new(r"^(((0[1-9]|[12][0-9]|3[01])[- /.](0[13578]|1[02])|(0[1-9]|[12][0-9]|30)[- /.](0[469]|11)|(0[1-9]|1\d|2[0-8])[- /.]02)[- /.]\d{4}|29[- /.]02[- /.](\d{2}(0[48]|[2468][048]|[13579][26])|([02468][048]|[1359][26])00))$").unwrap();

        //Correct dates for holidays that start before they end
        for holidayseason in &mut self.holiday_seasons {
            //Check start and end dates before parsing to naivedates
            if re.is_match(&holidayseason.start_date) && re.is_match(&holidayseason.end_date) {
                if NaiveDate::parse_from_str(&holidayseason.start_date, "%d/%m/%Y").unwrap()
                    > NaiveDate::parse_from_str(&holidayseason.end_date, "%d/%m/%Y").unwrap()
                {
                    //Correct start date to be a year earlier
                    let yearnow = Local::now().year();
                    holidayseason.start_date = holidayseason
                        .start_date
                        .trim_end_matches(&yearnow.to_string())
                        .to_string();
                    holidayseason.start_date += &(yearnow + -1).to_string();
                }
            }
        }

        //Check that isn't more than the number of seconds in a day and more than 0
        if self.howler_interval < 1 || self.howler_interval > 86349 {
            error!("Howler Interval ({}) Is not Valid! Must be above 0 and below 86349. Defaulting to 10 seconds!", self.howler_interval);
        }

        //Check public holidays and discard ones with invalid dates
        self.public_holidays.retain(|day| {
            if NaiveDate::parse_from_str(&day.date, "%d/%m/%Y").is_ok() {
                true
            } else {
                error!(
                    "Date ({}) of {} is not in valid DD/MM format! Ignoring this holiday!",
                    day.date, day.name
                );
                false
            }
        });

        //Check the holiday seasons and remove ones with invalid dates (start and end dates)
        self.holiday_seasons.retain(|holiday| {
            let start_datetime = NaiveDate::parse_from_str(&holiday.start_date, "%d/%m/%Y");
            let end_datetime = NaiveDate::parse_from_str(&holiday.end_date, "%d/%m/%Y");
            if start_datetime.is_ok(){
                if end_datetime.is_ok() {
                    // //While we're here issue a warning if the holiday ends before it starts (spans over the new year)
                    // if start_datetime.unwrap() > end_datetime.unwrap() {
                    //     warn!("{} ends before it starts! This season will be treated as spanning over the new year!", holiday.name);
                    // }
                    true
                } else {
                    //The configuration for this holiday is invalid so discard it
                    error!("End date ({}) of {} is not in valid DD/MM format! Ignoring this holiday season!", holiday.end_date, holiday.name);
                    false
                }
            } else {
                //The configuration for this holiday is invalid so discard it
                error!("Start date ({}) of {} is not in valid DD/MM format! Ignoring this holiday season!", holiday.start_date, holiday.name);
                false
            }
        });

        let mut remove = Vec::new();

        for (i, user) in self.users.iter().enumerate() {
            let button_pin = user.pin_config.button;
            if !used_pins.contains(&button_pin) && ALLOWED_PINS.contains(&button_pin) {
                used_pins.push(button_pin);
            } else {
                remove.push(i);
                error!(
                    "{}",
                    Error::Pin {
                        t: String::from("Button"),
                        name: user.name.clone(),
                        pin: button_pin,
                    }
                );
            }

            let led_pin = user.pin_config.led;
            if !used_pins.contains(&led_pin) && ALLOWED_PINS.contains(&led_pin) {
                used_pins.push(led_pin);
            } else {
                remove.push(i);
                error!(
                    "{}",
                    Error::Pin {
                        t: String::from("LED"),
                        name: user.name.clone(),
                        pin: led_pin,
                    }
                );
            }
        }

        // Remove the invalid users
        remove.iter().rev().for_each(|i| {
            self.users.remove(*i);
        });

        //Check all reminders for valid times
        for i in 0..self.users.len() {
            let user_name = self.users[i].name.clone();
            //Check reminders
            self.users[i].reminders.retain(|reminder| {
                if NaiveTime::parse_from_str(&reminder.light_on, "%H:%M").is_ok() {
                    true
                } else {
                    error!("Reminder {} from user {}'s time is not in valid HH:MM format! Ignoring this reminder!", &reminder.label, &user_name);
                    info!("{} {}", &reminder.light_on, "%H:%M");
                    false
                }
            });
            //Sort the reminders by their time
            self.users[i]
                .reminders
                .sort_by(|a, b| a.light_on.cmp(&b.light_on));
        }
    }
}

//Old code for above

//Check the holiday seasons and remove invalid ones
// for holiday in &self.holiday_seasons {
//     let start_date = DateTime::parse_from_str(&holiday.start_date, "%d/%m");
//     let end_date = DateTime::parse_from_str(&holiday.end_date, "%d/%m");

//     //If the start date is invalid
//     if start_date.is_err() {
//         error!("Start date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday season!", holiday.start_date, holiday.name);

//         //Find this holiday season and remove it
//         let index = self.holiday_seasons.iter().position(|x| (&x.start_date == &holiday.start_date && &x.name == &holiday.name)).unwrap();
//         self.holiday_seasons.remove(index);
//     }
//     //If the dates are invalid
//     else if end_date.is_err() {
//         error!("End date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday season!", holiday.end_date, holiday.name);

//         //Find this holiday season and remove it
//         let index = self.holiday_seasons.iter().position(|&x| (x.end_date == holiday.end_date && x.name == holiday.name)).unwrap();
//         self.holiday_seasons.remove(index);
//     }
// }
