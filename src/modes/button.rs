#[derive(Clone, Copy, PartialEq)]
pub struct Button {
    state: bool,
}

impl Button {
    pub fn new() -> Self {
        Button { state: false }
    }

    pub fn is_on(&self) -> bool {
        self.state
    }

    pub fn set(&mut self, new_state: bool) {
        self.state = new_state;
    }

    pub fn toggle(&mut self) -> bool {
        self.state = !self.state;
        self.state
    }
}

impl Default for Button {
    fn default() -> Self {
        Self::new()
    }
}
