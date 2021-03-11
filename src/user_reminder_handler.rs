use std;
use std::sync::mpsc;
use chrono::prelude::*;
use std::thread::sleep;
use rust_gpiozero::*;
use super::reminder_howler::HowlerUpdatePacket;
use super::conf;

/*
This file contains the code run on the threads dedicated to each user.
It handles the leds
*/

pub fn start (user_id: u8, holiday: bool, tx_howler: mpsc::Sender<HowlerUpdatePacket>)
{

    let user = &conf::CONFIG.users[user_id as usize];

    //Get reminders depending on whether today is a holiday
    let reminders;

    //Init LED
    let led = LED::new(user.pin_config.led);

    //Channels to talk to and recieve from button listener
    //let (tx_button_listener, rx_button_listener) = mpsc::channel();

    //Start a child thread to listen to the user's button and report to the howler when it is pressed
    //Clone sender
    let tx_button = tx_howler.clone();
    let buttonpin = user.pin_config.button;
    std::thread::spawn(move || {
        //Init Button
        let mut button = Button::new(buttonpin);
        
        while true {
            button.wait_for_press(std::option::Option::None);
            //Set the active reminder to null
            tx_button.send(HowlerUpdatePacket { job: super::reminder_howler::UpdateCommand::Set, user_id: user_id, reminder_label: String::new() });
        }
    });

    //Use holiday reminders if it is a holiday otherwise use normal reminders
    if holiday { reminders = &user.reminders_h; } else { reminders = &user.reminders }

    //Loop through reminders (they will already be sorted by their time)
    for rem in reminders {
        let rem_time = NaiveTime::parse_from_str(&rem.light_on, "%H:%M").unwrap();
        let now = Local::now().time();

        //If reminder has passed already (by more than one second) skip it (probably because the program has just been started)
        if rem_time < now && now > rem_time.with_second(1).expect("Reminder time is invalid! Something has gone VERY wrong!"){
            continue;
        }
        else {
            //If we aren't running late sleep until the reminder is due
            if rem_time > now {
                let timeUntilReminder = rem_time.signed_duration_since(now).to_std().unwrap();
                info!("USER {}: Next Reminder ({}) in {} seconds!", &user.name, &rem.label, timeUntilReminder.as_secs());
                sleep(timeUntilReminder);
            }

            //Turn on the LED
            led.on();
            
            //Spawn a thread to update the active reminder before the grace period and after assuming another reminder hasn't fired
            //Make a clone of tx_howler for sending messages from this thread
            let tx_grace_period = tx_howler.clone();
            let grace_period: u64 = rem.grace_period.into();
            let reminder_label = rem.label.clone();
            std::thread::spawn(move || {
                //Set the active reminder to this reminder and disable audio
                tx_grace_period.send(HowlerUpdatePacket { job: super::reminder_howler::UpdateCommand::Set, user_id: user_id, reminder_label: reminder_label.clone() });
                //Sleep for the duration of the grace period
                sleep(std::time::Duration::new(grace_period, 0));
                //Enable the audio for this reminder if it is still the active reminder
                tx_grace_period.send(HowlerUpdatePacket { job: super::reminder_howler::UpdateCommand::Enable, user_id: user_id, reminder_label: reminder_label.clone() });
            });

        }

    }
}