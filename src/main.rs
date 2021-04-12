use chrono::prelude::*;
use lazy_static::lazy_static;
use std::env;
use std::fs;
use std::process::Command;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::thread::sleep;

mod conf;
mod reminder_howler;
mod user_reminder_handler;

#[macro_use]
extern crate log;
lazy_static! {
    static ref THREAD_HANDLES: std::sync::RwLock<Vec<Option<thread::JoinHandle<()>>>> = std::sync::RwLock::new(std::iter::repeat_with(|| None).take(conf::CONFIG.users.len()).collect()); 
    // pub static ref KILL_CODE: Vec<std::thread::JoinHandle<()>> = Vec::new().resize(conf::CONFIG.users.len(), std::thread::JoinHandle<()>);
    //CONDVAR STUFF!!!
    pub static ref KILL_CODE: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
}
fn main() {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
    );
    pretty_env_logger::init();

    //Begin Log
    info!(
        "HOST: Starting lightbox version {}",
        env!("CARGO_PKG_VERSION")
    );

    loop {
        //Create the folder for the tts audio files
        if fs::metadata("remindersounds").is_ok() && fs::create_dir("remindersounds").is_err() {
            panic!("HOST: Insufficient permissions to create audio folder! Ensure this user has permissions to create files in this directory");
        }

        //while true {
        //Check if today is a holiday and remember that
        let is_h = is_holiday();

        //Start Howler with message link for reminder_handler --> howler communication
        let (tx_reminder_handler_sender, rx_howler_listener) = mpsc::channel();
        let howl_interval = conf::CONFIG.howler_interval;
        let user_count = conf::CONFIG.users.len();
        {
            THREAD_HANDLES.write().unwrap().push(Some(thread::spawn(move || {
                reminder_howler::start_howler(rx_howler_listener, howl_interval, user_count);
            })));
        }   

        //TODO: Run tts conversions for today's reminders WITHIN RUST
        for (i, user) in conf::CONFIG.users.iter().enumerate() {
            //Declare binding for today's reminders
            let reminders_today;

            //Cache the reminders for today
            if is_h {
                reminders_today = &user.reminders_h;
            } else {
                reminders_today = &user.reminders;
            }

            //Convert to sound and store in ./remindersounds
            for reminder in reminders_today {
                //TODO: CONVERT USING RUST
                //NOTE: This is a janky workaround using python because I can't get any of the rust bindings to work on ARM64 for the time being
                Command::new("sh").args(&[
                    "python3",
                    "./src/ttsjank.py",
                    &conf::CONFIG.tts_lan,
                    //Use vocal_reminder if present, fall back to label if not
                    match &reminder.vocal_reminder {
                        Some(p) => p,
                        None => &reminder.label,
                    },
                    &("./remindersounds/".to_owned() + &reminder.label.clone()),
                ]);
            }

            //TODO: Launch worker thread
            let this_tx_reminder_handler = tx_reminder_handler_sender.clone();
            {
                THREAD_HANDLES.write().unwrap().push(Some(thread::spawn(move || {
                    user_reminder_handler::start(i as u8, is_h, this_tx_reminder_handler);
                })));
            }
        }

        //Sleep until the next day (0:00)
        let now = Utc::now().naive_local();
        //Calculate time one day in the future when H M S are all 0
        let next_day = NaiveDateTime::new(
            now.date() + chrono::Duration::days(1),
            NaiveTime::from_hms(0, 0, 0),
        );
        //Get the difference
        let time_until_next_day = next_day.signed_duration_since(now).to_std().unwrap();
        //Sleep until the next day
        sleep(time_until_next_day);

        //Kill all the threads if they haven't left already
        let mut kill_mutex = KILL_CODE.0.lock().unwrap();
        *kill_mutex = true;
        //Notify threads
        KILL_CODE.1.notify_all();
        //Join them to make sure they are dead
        let mut handles = THREAD_HANDLES.write().unwrap();
        for i in 0..handles.len() {
            match handles.remove(i) {
                Some(h) => h.join().unwrap(),
                None => continue
            }
        }
    }
} 

    //Figure out if today is a holiday
fn is_holiday() -> bool {
    //Get current date and time
        let now = Local::now();

    //Is today a public holiday
    if conf::CONFIG
        .public_holidays
        .iter()
        .any(|day| day.date == now.format("%d/%m").to_string())
    {
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

    false
}
