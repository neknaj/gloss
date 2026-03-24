use alloc::string::String;

pub struct Preedit {
    pub text: String,
}

pub struct ImeState {
    pub active: bool,
}

impl ImeState {
    pub fn new() -> Self {
        ImeState { active: false }
    }
}

impl Default for ImeState {
    fn default() -> Self { Self::new() }
}
