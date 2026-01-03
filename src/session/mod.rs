use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub last_cwd: String,
    // Add other session data
}

pub fn save_session(_state: &SessionState) {
    // Implement save logic
}

pub fn load_session() -> SessionState {
    // Implement load logic
    SessionState::default()
}
