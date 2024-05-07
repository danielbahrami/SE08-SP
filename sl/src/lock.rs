use std::{sync::{Arc,mpsc::Receiver}, thread, time::Duration};
use std::collections::HashMap;
use std::sync::Mutex;
use esp_idf_svc::{
    hal::{
        gpio::OutputPin,
        ledc::{LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver,config::TimerConfig},
        peripheral::Peripheral,
        prelude::FromValueType
    },
    sys::EspError
};
use log::info;
use crate::state::{make_blue, make_green, make_orange, make_red, make_yellow, State};


#[derive(Clone, Copy)]
pub struct SmartLock {
    state: State,
}


impl SmartLock {
    pub fn new() -> SmartLock {
        SmartLock {
            state: State::NONE,
        }
    }

    pub fn set_state(&mut self, state: State) {
        self.state = state;
    }

    pub fn get_state(&self) -> State {
        self.state
    }
    pub fn setup_leds(
        timer0: impl Peripheral<P = impl LedcTimer> + 'static,
        channel0: impl Peripheral<P = impl LedcChannel> + 'static,
        channel1: impl Peripheral<P = impl LedcChannel> + 'static,
        channel2: impl Peripheral<P = impl LedcChannel> + 'static,
        pin25: impl Peripheral<P = impl OutputPin> + 'static,
        pin32: impl Peripheral<P = impl OutputPin> + 'static,
        pin33: impl Peripheral<P = impl OutputPin> + 'static
    ) -> Result<(LedcDriver<'static>, LedcDriver<'static>, LedcDriver<'static>), EspError> {
        let led_timer = Arc::new(LedcTimerDriver::new(
            timer0,
            &TimerConfig::default().frequency(25.kHz().into())
        )?);
        let red_pin = LedcDriver::new(
            channel0,
            led_timer.clone(),
            pin25
        )?;
        let green_pin = LedcDriver::new(
            channel1,
            led_timer.clone(),
            pin32
        )?;
        let blue_pin = LedcDriver::new(
            channel2,
            led_timer.clone(),
            pin33
        )?;
        Ok((red_pin, green_pin, blue_pin))
    }

    pub fn run(
        smart_lock: Arc<Mutex<SmartLock>>,
        state_rx: Receiver<State>,
        mut red_pin: LedcDriver<'static>,
        mut green_pin: LedcDriver<'static>,
        mut blue_pin: LedcDriver<'static>,

    ) {
        let mut color: [u32; 3] = [0, 0, 0];
        let mut blink = false;
        thread::spawn(move || {
            loop {
                match state_rx.try_recv() {
                    Ok(state) => {
                        let mut lock_binding = smart_lock.lock().unwrap();
                        if state != lock_binding.get_state() {
                            info!("State {:?} => {:?}", lock_binding.get_state(), state);
                            lock_binding.set_state(state);
                            match state {
                                State::INITIALIZING => {
                                    blink = true;
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
}