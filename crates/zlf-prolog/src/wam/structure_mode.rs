use super::error::{WamError, WamResult};

#[derive(Debug, Clone, Default)]
pub enum StructureMode {
    #[default]
    None,
    Read {
        next_arg: usize,
        remaining: usize,
    },
    Write {
        remaining: usize,
    },
}

impl StructureMode {
    pub fn read(next_arg: usize, remaining: usize) -> Self {
        Self::Read {
            next_arg,
            remaining,
        }
    }

    pub fn write(remaining: usize) -> Self {
        Self::Write { remaining }
    }

    pub fn next_read_arg(&mut self) -> WamResult<usize> {
        match self {
            Self::Read {
                next_arg,
                remaining,
            } if *remaining > 0 => {
                let arg = *next_arg;
                *next_arg += 1;
                *remaining -= 1;
                Ok(arg)
            }
            _ => Err(WamError::InvalidInstructionState("read argument")),
        }
    }

    pub fn consume_write_arg(&mut self) -> WamResult<()> {
        match self {
            Self::Write { remaining } if *remaining > 0 => {
                *remaining -= 1;
                Ok(())
            }
            _ => Err(WamError::InvalidInstructionState("write argument")),
        }
    }

    pub fn maybe_consume_write_arg(&mut self) -> WamResult<()> {
        if matches!(self, Self::Write { .. }) {
            self.consume_write_arg()
        } else {
            Ok(())
        }
    }
}
