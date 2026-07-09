use super::environment::EnvironmentFrame;
use super::error::{WamError, WamResult};

#[derive(Debug, Default)]
pub struct EnvironmentStack {
    frames: Vec<EnvironmentFrame>,
}

impl EnvironmentStack {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn allocate(&mut self, continuation: Option<usize>) {
        let previous = self.frames.len().checked_sub(1);
        self.frames
            .push(EnvironmentFrame::allocate(&[], continuation, previous));
    }

    pub fn deallocate(&mut self) -> WamResult<()> {
        self.frames
            .pop()
            .map(|_| ())
            .ok_or(WamError::InvalidInstructionState("deallocate"))
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}
