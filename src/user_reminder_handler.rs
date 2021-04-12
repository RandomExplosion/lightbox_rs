use crate::conf;
use crate::reminder_howler::HowlerUpdatePacket;
use crate::THREAD_HANDLES;
use crate::KILL_CODE;
use chrono::prelude::*;
use rust_gpiozero::*;
use std::sync::{mpsc, Arc};

/*
This file contains the code run on the threads dedicated to each user.
It handles the leds
*/

pub fn start(user_id: u8, holiday: bool, tx_howler: mpsc::Sender<HowlerUpdatePacket>) {
    //Get the user
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
    {   
        THREAD_HANDLES.write().unwrap().push(Some(std::thread::spawn(move || {
            //Init Button
            let mut button = Button::new(buttonpin);
    
            loop {
                button.wait_for_press(None);
                //Set the active reminder to null
                tx_button
                    .send(HowlerUpdatePacket {
                        job: super::reminder_howler::UpdateCommand::Set,
                        user_id,
                        reminder_label: String::new(),
                    })
                    .unwrap();
            }
        })));
    }

    //Use holiday reminders if it is a holiday otherwise use normal reminders
    if holiday {
        reminders = &user.reminders_h;
    } else {
        reminders = &user.reminders
    }

    //Loop through reminders (they will already be sorted by their time)
    for rem in reminders {
        let rem_time = NaiveTime::parse_from_str(&rem.light_on, "%H:%M").unwrap();
        let now = Local::now().time();
        {
            //Get the killcode mutex and condvar
            let (kill_lock, kill_condvar) = &*Arc::clone(&KILL_CODE);

            let shouldkill = kill_lock.lock().unwrap();
            //If we aren't running late sleep until the reminder is due
            if rem_time > now {
                let time_until_reminder = rem_time.signed_duration_since(now).to_std().unwrap();
                info!(
                    "USER {}: Next Reminder ({}) in {} seconds!",
                    &user.name,
                    &rem.label,
                    time_until_reminder.as_secs()
                );
                let result = kill_condvar.wait_timeout(shouldkill, time_until_reminder).unwrap();
                //If this thread was woken up
                if *result.0 == true {
                    return;
                }
            }
        }
        //Turn on the LED
        led.on();

        //Spawn a thread to update the active reminder before the grace period and after assuming another reminder hasn't fired
        //Make a clone of tx_howler for sending messages from this thread
        let tx_grace_period = tx_howler.clone();
        let grace_period: u64 = rem.grace_period.into();
        let reminder_label = rem.label.clone(); 
        THREAD_HANDLES.write().unwrap().push(Some(std::thread::spawn(move || {
            
            //Get the killcode mutex and condvar
            let (kill_lock, kill_condvar) = &*Arc::clone(&KILL_CODE);    

            //Set the active reminder to this reminder and disable audio
            tx_grace_period
                .send(HowlerUpdatePacket {
                    job: super::reminder_howler::UpdateCommand::Set,
                    user_id,
                    reminder_label: reminder_label.clone(),
                })
                .unwrap();
            //Sleep for the duration of the grace period
            let _ = kill_condvar.wait_timeout(kill_lock.lock().unwrap(), std::time::Duration::new(grace_period, 0));
            //Enable the audio for this reminder if it is still the active reminder
            tx_grace_period
                .send(HowlerUpdatePacket {
                    job: super::reminder_howler::UpdateCommand::Enable,
                    user_id,
                    reminder_label: reminder_label.clone(),
                })
                .unwrap();
        })));
    }
}
