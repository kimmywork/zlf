use super::environment::EnvironmentFrame;
use super::error::{WamError, WamResult};

#[derive(Debug, Default, Clone)]
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

    pub fn allocate(
        &mut self,
        continuation: Option<usize>,
        cut_base: usize,
        permanent_count: usize,
    ) {
        let previous = self.frames.len().checked_sub(1);
        self.frames.push(EnvironmentFrame::allocate(
            &[],
            continuation,
            previous,
            cut_base,
            permanent_count,
        ));
    }

    pub fn deallocate(&mut self) -> WamResult<()> {
        self.frames
            .pop()
            .map(|_| ())
            .ok_or(WamError::InvalidInstructionState("deallocate"))
    }

    pub fn cut_base(&self) -> Option<usize> {
        self.frames.last().map(EnvironmentFrame::cut_base)
    }

    pub fn permanent_slot(&self, slot: usize) -> WamResult<Option<usize>> {
        self.frames
            .last()
            .and_then(|frame| frame.permanent_slot(slot))
            .ok_or(WamError::InvalidInstructionState("permanent slot"))
    }

    pub fn set_permanent_slot(&mut self, slot: usize, addr: usize) -> WamResult<()> {
        if let Some(frame) = self.frames.last_mut() {
            if frame.set_permanent_slot(slot, addr) {
                return Ok(());
            }
        }
        Err(WamError::InvalidInstructionState("permanent slot"))
    }

    pub fn cut_level(&self, slot: usize) -> WamResult<Option<usize>> {
        self.frames
            .last()
            .and_then(|frame| frame.cut_level(slot))
            .ok_or(WamError::InvalidInstructionState("cut level"))
    }

    pub fn set_cut_level(&mut self, slot: usize, level: usize) -> WamResult<()> {
        if let Some(frame) = self.frames.last_mut() {
            if frame.set_cut_level(slot, level) {
                return Ok(());
            }
        }
        Err(WamError::InvalidInstructionState("cut level"))
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}
