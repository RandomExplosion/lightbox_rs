use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc;
use std::thread::sleep;

//Packet task enum (see HowlerUpdatePacket) for purpose
#[derive(PartialEq)]
pub enum UpdateCommand {
    Set,
    Enable,
}

///
/// This one's a bit hard to explain so bear with me.
/// The user reminder handlers can do one of two things when communicating with the howler depending on the value of job
/// 1. UpdateCommand::Set will change the user's active reminder id string to reminder_label and disable audio
/// 2. UpdateCommand::Enable will enable audio only if reminder_label matches the senders actove reminder id
/// TODO: Merge this struct into UpdateCommand
///
pub struct HowlerUpdatePacket {
    pub job: UpdateCommand,
    pub user_id: u8,
    pub reminder_label: String,
}

//Struct to store the active reminder for a user and whether sound should be played or not
#[derive(Default, Clone)]
pub struct UserActiveReminder {
    reminder_label: String,
    audio_active: bool,
}

//Function to start the howler process
pub fn start_howler(
    rx_reminder_updates: mpsc::Receiver<HowlerUpdatePacket>,
    howl_interval: u32,
    user_count: usize,
) {
    info!("HOWLER: Starting Howler!");
    info!("HOWLER: Initialising audio sink...");
    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    info!("HOWLER: Sink Initialised!");
    info!(
        "HOWLER: Starting Playback Loop with interim delay of {}s",
        howl_interval
    );

    //Create a vector to store the active reminders for each user
    let mut active_reminders: Vec<UserActiveReminder> = vec![Default::default(); user_count];

    //Runtime Loop
    loop {
        //Get an iterator for all messages recieved since the last playback.
        let update_iter = rx_reminder_updates.try_iter();

        //Process packets
        for update in update_iter {
            active_reminders[update.user_id as usize] = match update.job {
                UpdateCommand::Set => UserActiveReminder {
                    reminder_label: update.reminder_label,
                    audio_active: false,
                },
                UpdateCommand::Enable => UserActiveReminder {
                    reminder_label: active_reminders[update.user_id as usize]
                        .reminder_label
                        .clone(),
                    audio_active: true,
                },
            }
        }

        let sound_path = "./remindersounds/";

        //Loop through and add all active sounds to the sink
        for reminder in &active_reminders {
            if reminder.reminder_label.trim().is_empty() {
                continue;
            } else {
                let file = File::open(sound_path.to_owned() + &reminder.reminder_label).unwrap();
                let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                sink.append(source);
            }
        }
        //TODO: Play sounds in sink
        sink.play();
        sink.sleep_until_end();

        //Sleep for designated time then rinse and repeat
        sleep(std::time::Duration::from_secs(howl_interval.into()));
    }
}
