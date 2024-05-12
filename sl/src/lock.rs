use crate::state::{State, StateNode};
use esp_idf_svc::{
    hal::{
        gpio::OutputPin,
        ledc::{config::TimerConfig, LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver},
        peripheral::Peripheral,
        prelude::FromValueType,
    },
    sys::EspError,
};

use crate::state::State::ERROR;
use std::collections::HashMap;
use std::sync::Mutex;
use std::{
    sync::{mpsc::Receiver, mpsc::Sender, Arc},
    thread,
    time::Duration,
};

pub struct SmartLock {
    state: State,
    error_condition: Option<String>,
    transitions: HashMap<State, StateNode>,
    sim_tx: Option<Sender<String>>,
}

impl SmartLock {
    pub fn new() -> SmartLock {
        SmartLock {
            state: State::NONE,
            error_condition: None,
            transitions: HashMap::new(),
            sim_tx: None,
        }
    }

    /// Add a transition to a new state
    pub fn add_transition(
        &mut self,
        given_state: State,
        event: &str,
        next_state: State,
    ) -> &mut Self {
        self.transitions
            .entry(given_state)
            .and_modify(|node| {
                node.add_state_transition(event.to_string(), next_state);
            })
            .or_insert(StateNode::new_with_transition(
                event.to_string(),
                next_state,
            ));
        self
    }

    /// Add a transitions that simulates a second event after some delay
    pub fn add_sim_transition(
        &mut self,
        given_state: State,
        event: &str,
        intermediate_state: State,
        sim_delay: u64,
        sim_event: &str,
    ) -> &mut Self {
        self.transitions
            .entry(given_state)
            .and_modify(|node| {
                node.add_sim_state_transition(
                    event.to_string(),
                    intermediate_state,
                    sim_delay,
                    sim_event,
                );
            })
            .or_insert(StateNode::new_with_sim_transition(
                event.to_string(),
                intermediate_state,
                sim_delay,
                sim_event,
            ));
        self
    }

    /// Try to find a fitting state transition or sim transition
    fn try_transition(&mut self, event: String) {
        // Find the appropriate StateNode
        // |-> Look for normal transition in StateNode
        // |--> Set state from transition and return
        // |-> Look for sim transition in StateNode
        // |--> Set state from sim transition and spawn sim thread
        match self.transitions.get(&self.state) {
            None => return,
            Some(node) => match node.get_next_state(&event) {
                None => match node.get_next_sim_state(&event) {
                    None => return,
                    Some(sim) => {
                        let (intermediate_state, sim_delay, sim_event) = sim.clone();
                        self.set_state(intermediate_state, event.into());
                        // Simulate a delayed input, such as the locking mechanism
                        Self::spawn_sim_thread(sim_delay, sim_event, self.sim_tx.clone());
                    }
                },
                Some(next_state) => {
                    self.set_state(*next_state, event.into());
                }
            },
        }
    }

    fn spawn_sim_thread(sim_delay: u64, sim_event: String, tx: Option<Sender<String>>) {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(sim_delay));
            tx.unwrap().send(sim_event).unwrap();
        });
    }

    /// Link channel Sender. Used for sim threads
    pub fn link_channel(&mut self, tx: Option<Sender<String>>) {
        self.sim_tx = tx;
    }

    fn set_state(&mut self, state: State, event: Option<String>) {
        if state == ERROR {
            self.error_condition = event;
        } else {
            self.error_condition = None;
        }
        self.state = state;
    }

    pub(crate) fn get_state(&self) -> State {
        self.state
    }
    pub fn setup_led(
        timer0: impl Peripheral<P = impl LedcTimer> + 'static,
        channel0: impl Peripheral<P = impl LedcChannel> + 'static,
        channel1: impl Peripheral<P = impl LedcChannel> + 'static,
        channel2: impl Peripheral<P = impl LedcChannel> + 'static,
        pin25: impl Peripheral<P = impl OutputPin> + 'static,
        pin32: impl Peripheral<P = impl OutputPin> + 'static,
        pin33: impl Peripheral<P = impl OutputPin> + 'static,
    ) -> Result<
        (
            LedcDriver<'static>,
            LedcDriver<'static>,
            LedcDriver<'static>,
        ),
        EspError,
    > {
        let led_timer = Arc::new(LedcTimerDriver::new(
            timer0,
            &TimerConfig::default().frequency(25.kHz().into()),
        )?);
        let red_pin = LedcDriver::new(channel0, led_timer.clone(), pin25)?;
        let green_pin = LedcDriver::new(channel1, led_timer.clone(), pin32)?;
        let blue_pin = LedcDriver::new(channel2, led_timer.clone(), pin33)?;
        Ok((red_pin, green_pin, blue_pin))
    }

    pub fn run(
        smart_lock: Arc<Mutex<SmartLock>>,
        event_rx: Receiver<String>,
        mut red_pin: LedcDriver<'static>,
        mut green_pin: LedcDriver<'static>,
        mut blue_pin: LedcDriver<'static>,
    ) {
        let mut color: [u32; 3] = [0, 0, 0];
        let mut blink = false;
        thread::spawn(move || loop {
            // Try to get any updates from MQTT or sim threads
            match event_rx.try_recv() {
                Ok(event) => {
                    let mut lock_binding = smart_lock.lock().unwrap();
                    lock_binding.try_transition(event);
                    lock_binding.get_state().get_color(&mut color);
                    blink = lock_binding.error_condition.is_some();
                }
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
        });
    }
}
