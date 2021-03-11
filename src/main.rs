use chrono::prelude::*;
use std::env;
use pretty_env_logger;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use chrono::prelude::*;
use std::thread::sleep;
use std::fs;

mod conf;
mod user_reminder_handler;
mod reminder_howler;

#[macro_use] extern crate log;

fn main() {
    
    env::set_var("RUST_LOG", env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()));
    pretty_env_logger::init();

    //Begin Log
    info!("HOST: Starting lightbox version {}", env!("CARGO_PKG_VERSION"));


    loop {

        //Create the folder for the tts audio files 
        if fs::metadata("remindersounds").is_ok() {
            if fs::create_dir("remindersounds").is_err() {
                panic!("HOST: Insufficient permissions to manipulate tts sound files! Try running the program with sudo.");
            }
        }

        //while true {
        //Check if today is a holiday and remember that
        let is_h = is_holiday();
        
        //Start Howler with message link for reminder_handler --> howler communication
        let (tx_reminder_handler_sender, rx_howler_listener) = mpsc::channel();
        let howl_interval = conf::CONFIG.howler_interval;
        let user_count: usize = conf::CONFIG.users.len().into();
        thread::spawn(move || {
            reminder_howler::start_howler(rx_howler_listener, howl_interval, user_count);
        });

        let mut i = 0;

        //TODO: Run tts conversions for today's reminders WITHIN RUST
        for mut user in &conf::CONFIG.users {

            //Declare binding for today's reminders
            let reminders_today;

            //Cache the reminders for today
            if is_h { reminders_today = &user.reminders_h; } else { reminders_today = &user.reminders; }

            //Convert to sound and store in ./remindersounds
            for reminder in reminders_today {
                //TODO: CONVERT USING RUST
                //NOTE: This is a janky workaround using python because I can't get any of the rust bindings to work on ARM64 for the time being
                Command::new("sh").args(&["python3", &conf::CONFIG.tts_lan, 
                    //Use vocal_reminder if present, fall back to label if not
                    match &reminder.vocal_reminder {
                        Some(p) => p,
                        None => &reminder.label,
                    }, 
                    
                    &("./remindersounds/".to_owned() + &reminder.label.clone())]);
            }

            //TODO: Launch worker thread
            let this_tx_reminder_handler = tx_reminder_handler_sender.clone();
            thread::spawn(move | | {
                user_reminder_handler::start(i, is_h, this_tx_reminder_handler);
            });

            i = i+1;
        }

        //Sleep until the next day (0:00)
        let now = Utc::now().naive_local();
        //Calculate time one day in the future when H M S are all 0
        let next_day = NaiveDateTime::new(now.date()+chrono::Duration::days(1), NaiveTime::from_hms(0, 0, 0));
        //Get the difference
        let time_until_next_day = next_day.signed_duration_since(now).to_std().unwrap();
        //Sleep until the next day
        sleep(time_until_next_day);
    }
}

//Figure out if today is a holiday
fn is_holiday() -> bool {

    //Get current date and time
    let now = Local::now();

    //Is today a weekend?
    if now.format("%a").to_string() == "Sat" || now.format("%a").to_string() == "Sun" {
        info!("Today is a weekend!\n Using holiday reminders.");
        return true;
    }

    //Is today a public holiday
    if conf::CONFIG.public_holidays.iter().any(|day| &day.date == &now.format("%d/%m").to_string()) {
        info!("Today is a public holiday!\nUsing holiday reminders.");
    }
    //Or More readably:
    // for day in conf::CONFIG.public_holidays {
    //     if now.format("%d/%m").to_string() == day.date {
    //         info!("Today is {}!\nUsing holiday reminders", day.name);
    //     }
    // }

    for holiday in &conf::CONFIG.holiday_seasons {
        //Parse start and end dates from strings to NaiveDates for comparison;
        let start_date = NaiveDate::parse_from_str(&holiday.start_date, "%d/%m/%Y").unwrap();
        let end_date = NaiveDate::parse_from_str(&holiday.end_date, "%d/%m/%Y").unwrap();
        
        //Check if today falls between the start and end dates
        if start_date <= now.naive_local().date() && now.naive_local().date() <= end_date {
            info!("It's the {0}!\nUsing holiday reminders.", holiday.name);
            return true;
        }
    }

    return false;
}