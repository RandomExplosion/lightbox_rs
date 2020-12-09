use serde::{Deserialize, Serialize};
use log::{info, trace, warn, error};
use chrono::prelude::*;

const ALLOWED_PINS: [u8; 25] = [2,3,4,17,27,22,10,9,11,5,6,13,19,26,18,23,24,25,8,7,1,12,16,20,21];

/*
*   This file contains the struct equivalents of the json object in the config file
*   For more info see ConfigSchema.json
*/

#[derive(Serialize, Deserialize)]
pub struct HolidaySeason {
    pub name: String,
    pub start_date: String,
    pub end_date: String
}

#[derive(Serialize, Deserialize)]
pub struct Holiday {
    pub name: String,
    pub date: String
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
    pub label: String,
    pub vocal_reminder: String,
    pub light_on: String,
    pub grace_period: u8
}

#[derive(Serialize, Deserialize)]
pub struct PinConfig 
{
    pub button: u8,
    pub led: u8
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub reminders: Vec<Reminder>,
    pub reminders_h: Vec<Reminder>,
    pub pin_config: PinConfig
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub tts_lan: String,
    pub snooze_pin: u8,
    pub public_holidays: Vec<Holiday>,
    pub holiday_seasons: Vec<HolidaySeason>,
    pub users: Vec<User>
}

//Checks that configuration is valid and removes all invalid entries
pub fn validate_config (mut config: Configuration) {
    //TODO: Check tts language
    let mut used_pins: Vec<u8> = Vec::new();

    //Check public holidays and discard ones with invalid dates
    config.public_holidays = config.public_holidays.into_iter().filter(|day| {
        if DateTime::parse_from_str(&day.date, "%d/%m").is_ok() {
            true
        } else {
            error!("Date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday!", day.date, day.name);
            false
        }
    }).collect();

    //Check the holiday seasons and remove ones with invalid dates (start and end dates)
    config.holiday_seasons = config.holiday_seasons.into_iter().filter(|holiday| {
        let start_datetime = DateTime::parse_from_str(&holiday.start_date, "%d/%m");
        let end_datetime = DateTime::parse_from_str(&holiday.end_date, "%d/%m");
        if start_datetime.is_ok(){
            if end_datetime.is_ok() {
                //While we're here issue a warning if the holiday ends before it starts (spans over the new year)
                if start_datetime.unwrap() > end_datetime.unwrap() {
                    warn!("{} ends before it starts! This season will be treated as spanning over the new year!", holiday.name);
                }

                //This one's all good!
                true
            } else {
                //The configuration for this holiday is invalid so discard it
                error!("End date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday season!", holiday.end_date, holiday.name);
                false
            }
        } else {
            //The configuration for this holiday is invalid so discard it
            error!("Start date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday season!", holiday.start_date, holiday.name);
            false
        }
    }).collect();
    
    //TODO: Check all users for valid pins
    config.users = config.users.into_iter().filter(|user| {
        //Button pin must not be used by another user (or the snooze button)
        if !used_pins.contains(&user.pin_config.button) {
            //Button pin must be on the allowed list
            if ALLOWED_PINS.contains(&user.pin_config.button) {
                used_pins.push(user.pin_config.button);
                //LED pin must not be used by another user (or the snooze button)
                if !used_pins.contains(&user.pin_config.led) {
                    //LED pin must be on the allowed list
                    if ALLOWED_PINS.contains(&user.pin_config.led) {
                        used_pins.push(user.pin_config.led);
                        true
                    }
                    else {
                        error!("{}'s LED cannot be attached to pin {} because it is reserved for other uses.\nReminders for this user have been disabled.", &user.name, &user.pin_config.led);
                        false
                    }
                } else {
                    error!("{}'s LED cannot be attached to pin {} because it is in use by another user.\nReminders for this user have been disabled.", &user.name, &user.pin_config.led);
                    false
                }
            }
            else {
                error!("{}'s Button cannot be attached to pin {} because it is reserved for other uses.\nReminders for this user have been disabled.", &user.name, &user.pin_config.button);
                false
            }
        } else {
            error!("{}'s LED cannot be attached to pin {} because it is in use by another user.\nReminders for this user have been disabled.", &user.name, &user.pin_config.button);
            false
        }
    }).collect();

    //TODO: Check all alarms for valid dates
    for mut user in config.users {
        //Check reminders
        user.reminders = user.reminders.into_iter().filter(|reminder| {
            if DateTime::parse_from_str(&reminder.light_on, "%d/%m").is_ok() {
                true
            } else {
                //error!("Reminder");
                false
            }
        }).collect();
    }

}

//Old code for above

    //Check the holiday seasons and remove invalid ones
    // for holiday in &config.holiday_seasons {
    //     let start_date = DateTime::parse_from_str(&holiday.start_date, "%d/%m");
    //     let end_date = DateTime::parse_from_str(&holiday.end_date, "%d/%m");
        
    //     //If the start date is invalid
    //     if start_date.is_err() {
    //         error!("Start date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday season!", holiday.start_date, holiday.name);

    //         //Find this holiday season and remove it
    //         let index = config.holiday_seasons.iter().position(|x| (&x.start_date == &holiday.start_date && &x.name == &holiday.name)).unwrap();
    //         config.holiday_seasons.remove(index);
    //     } 
    //     //If the dates are invalid
    //     else if end_date.is_err() {
    //         error!("End date ({}) of {} is not in valid DD/MM format!\nIgnoring this holiday season!", holiday.end_date, holiday.name);

    //         //Find this holiday season and remove it
    //         let index = config.holiday_seasons.iter().position(|&x| (x.end_date == holiday.end_date && x.name == holiday.name)).unwrap();
    //         config.holiday_seasons.remove(index);
    //     }
    // }