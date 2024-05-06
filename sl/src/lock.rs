use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use esp_idf_svc::hal::ledc::LedcDriver;
use log::info;
use crate::state::{make_blue, make_green, make_orange, make_red, make_yellow, State};

pub fn run_leds(
    state_rx: Receiver<State>,
    mut red_pin: LedcDriver<'static>,
    mut green_pin: LedcDriver<'static>,
    mut blue_pin: LedcDriver<'static>) {
    let mut color: [u32; 3] = [0, 0, 0];
    let mut blink = false;
    thread::spawn(move || {
        let mut lock_state = State::NONE;
        loop {
            match state_rx.try_recv() {
                Ok(state) => {
                    if state != lock_state {
                        info!("State {:?} => {:?}", lock_state, state);
                        lock_state = state;
                        match state {
                            State::INITIALIZING => {
                                blink = false;
                                make_blue(&mut color);
                            },
                            State::CLOSED => {
                                blink = false;
                                make_red(&mut color);
                            },
                            State::CLOSING => {
                                blink = false;
                                make_orange(&mut color);
                            },
                            State::OPEN => {
                                blink = false;
                                make_green(&mut color);
                            },
                            State::OPENING => {
                                blink = false;
                                make_yellow(&mut color);
                            },
                            _ => {
                                blink = true;
                                make_red(&mut color);
                            }
                        }
                    }
                },
                Err(_) => {}
            }
            if blink {
                red_pin.set_duty(255).unwrap();
                green_pin.set_duty(255).unwrap();
                blue_pin.set_duty(255).unwrap();
                thread::sleep(Duration::from_millis(500));
            }
            red_pin.set_duty(255 - color[0]).unwrap();
            green_pin.set_duty(255 - color[1]).unwrap();
            blue_pin.set_duty(255 - color[2]).unwrap();
            thread::sleep(Duration::from_millis(500));
        }
    });
}