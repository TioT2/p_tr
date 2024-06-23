use std::collections::HashMap;

pub type KeyCode = winit::keyboard::KeyCode;


#[derive(Copy, Clone, PartialEq, Eq, Default, Hash)]
pub struct KeyState {
    pub pressed: bool,
    pub changed: bool,
}

pub struct State<'a> {
    states: &'a HashMap<KeyCode, KeyState>,
}

impl<'a> State<'a> {
    pub fn get_key_state(&self, key: KeyCode) -> KeyState {
        self.states
            .get(&key)
            .copied()
            .unwrap_or(KeyState::default())
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.get_key_state(key).pressed
    }

    pub fn is_key_clicked(&self, key: KeyCode) -> bool {
        let state = self.get_key_state(key);

        state.pressed && state.changed
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool {
        let state = self.get_key_state(key);

        !state.pressed && state.changed
    }
}

pub struct Input {
    key_states: HashMap<KeyCode, KeyState>,
}

impl Input {
    pub fn new() -> Input {
        Input {
            key_states: HashMap::new(),
        }
    }

    pub fn on_key_change(&mut self, key: KeyCode, new_pressed: bool) {
        let value = self.key_states.entry(key).or_insert(KeyState {
            pressed: !new_pressed,
            changed: false,
        });

        value.changed = value.pressed != new_pressed;
        value.pressed = new_pressed;
    }

    pub fn clear_changed(&mut self) {
        for state in self.key_states.values_mut() {
            state.changed = false;
        }
    }

    pub fn get_state<'a>(&'a self) -> State<'a> {
        State {
            states: &self.key_states,
        }
    }
}
