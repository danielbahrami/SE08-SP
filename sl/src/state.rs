use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum State {
    NONE,
    INITIALIZING,
    ERROR,
    LOCKED,
    LOCKING,
    UNLOCKED,
    UNLOCKING,
}

impl State {
    pub fn get_color(self, color: &mut [u32; 3]) {
        match self {
            State::NONE => {
                color[0] = 0;
                color[1] = 0;
                color[2] = 0;
            }
            State::INITIALIZING => {
                color[0] = 0;
                color[1] = 0;
                color[2] = 255;
            }
            State::LOCKED => {
                color[0] = 0;
                color[1] = 255;
                color[2] = 0;
            }
            State::UNLOCKED => {
                color[0] = 255;
                color[1] = 0;
                color[2] = 0;
            }
            State::LOCKING => {
                color[0] = 255;
                color[1] = 255;
                color[2] = 0;
            }
            State::UNLOCKING => {
                color[0] = 255;
                color[1] = 255;
                color[2] = 0;
            }
            State::ERROR => {
                color[0] = 255;
                color[1] = 0;
                color[2] = 0;
            }
        }
    }
}

pub struct StateNode {
    transitions: HashMap<String, State>,
    sim_transitions: HashMap<String, (State, u64, String)>,
}

impl StateNode {
    pub fn new_with_transition(event: String, next_state: State) -> StateNode {
        StateNode {
            transitions: HashMap::from([(event, next_state)]),
            sim_transitions: HashMap::new(),
        }
    }

    pub fn new_with_sim_transition(
        event: String,
        intermediate_state: State,
        sim_delay: u64,
        sim_event: &str,
    ) -> StateNode {
        StateNode {
            transitions: HashMap::new(),
            sim_transitions: HashMap::from([(
                event,
                (intermediate_state, sim_delay, sim_event.to_string()),
            )]),
        }
    }

    pub fn get_next_state(&self, event: &str) -> Option<&State> {
        self.transitions.get(event)
    }

    pub fn get_next_sim_state(&self, event: &str) -> Option<&(State, u64, String)> {
        self.sim_transitions.get(event)
    }

    pub fn add_state_transition(&mut self, event: String, next_state: State) {
        self.transitions.insert(event, next_state);
    }

    pub fn add_sim_state_transition(
        &mut self,
        event: String,
        intermediate_state: State,
        sim_delay: u64,
        next_state: &str,
    ) {
        self.sim_transitions.insert(
            event,
            (intermediate_state, sim_delay, next_state.to_string()),
        );
    }
}
