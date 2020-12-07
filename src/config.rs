use serde::{Deserialize, Serialize};

/*
*   This file contains the struct equivalents of the json object in the config file
*   For more info see ConfigSchema.json
*/

#[derive(Serialize, Deserialize)]
pub struct HolidaySeason {
    name: String,
    start_date: String,
    end_date: String
}

#[derive(Serialize, Deserialize)]
pub struct Holiday {
    name: String,
    date: String
}

#[derive(Serialize, Deserialize)]
pub struct Reminder {
    label: String,
    vocal_reminder: String,
    light_on: String,
    grace_period: u8
}

#[derive(Serialize, Deserialize)]
pub struct PinConfig 
{
    button: u8,
    led: u8
}

#[derive(Serialize, Deserialize)]
pub struct User {
    name: String,
    reminders: Vec<Reminder>,
    reminders_h: Vec<Reminder>,
    pin_config: PinConfig
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    tts_lan: String,
    snooze_pin: u8,
    users: Vec<User>
}
