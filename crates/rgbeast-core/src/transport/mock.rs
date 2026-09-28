//! Recording transports for tests and for the simulated daemon.

use std::collections::{HashMap, VecDeque};

use crate::{Error, Result};

use super::{HidTransport, Smbus};

/// Records every report written and replays queued replies.
#[derive(Default)]
pub struct MockHid {
    pub written: Vec<Vec<u8>>,
    pub replies: VecDeque<Vec<u8>>,
}

impl MockHid {
    pub fn with_replies(replies: Vec<Vec<u8>>) -> Self {
        MockHid {
            written: Vec::new(),
            replies: replies.into(),
        }
    }
}

impl HidTransport for MockHid {
    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.written.push(data.to_vec());
        Ok(())
    }
    fn read(&mut self, buf: &mut [u8], _timeout_ms: u32) -> Result<usize> {
        match self.replies.pop_front() {
            Some(r) => {
                let n = r.len().min(buf.len());
                buf[..n].copy_from_slice(&r[..n]);
                Ok(n)
            }
            None => Ok(0),
        }
    }
}

/// One recorded SMBus operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    ReadByte(u8),
    ReadByteData(u8, u8),
    ReadWordData(u8, u8),
    WriteByteData(u8, u8, u8),
    WriteWordData(u8, u8, u16),
    WriteBlockData(u8, u8, Vec<u8>),
}

/// A bus with a register file per address. Reads come from `regs`; writes
/// update `regs` and are logged in `ops`. Addresses absent from `present`
/// fail every operation, like an empty slot on a real bus.
#[derive(Default)]
pub struct MockSmbus {
    pub present: Vec<u8>,
    pub regs: HashMap<(u8, u8), u8>,
    pub words: HashMap<(u8, u8), u16>,
    pub ops: Vec<Op>,
    /// ENE-style indirect register file: (addr, 16-bit reg) -> value.
    pub ene: HashMap<(u8, u16), u8>,
    ene_pointer: HashMap<u8, u16>,
    /// Fail block writes (to exercise the byte-wise fallback).
    pub reject_blocks: bool,
}

impl MockSmbus {
    pub fn with_addresses(present: &[u8]) -> Self {
        MockSmbus {
            present: present.to_vec(),
            ..Default::default()
        }
    }

    fn check(&self, addr: u8) -> Result<()> {
        if self.present.contains(&addr) {
            Ok(())
        } else {
            Err(Error::Smbus(format!("no device at 0x{addr:02X}")))
        }
    }

    pub fn writes(&self) -> Vec<&Op> {
        self.ops
            .iter()
            .filter(|o| {
                !matches!(
                    o,
                    Op::ReadByte(_) | Op::ReadByteData(..) | Op::ReadWordData(..)
                )
            })
            .collect()
    }
}

impl Smbus for MockSmbus {
    fn read_byte(&mut self, addr: u8) -> Result<u8> {
        self.check(addr)?;
        self.ops.push(Op::ReadByte(addr));
        Ok(0)
    }
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8> {
        self.check(addr)?;
        self.ops.push(Op::ReadByteData(addr, reg));
        // ENE indirect read: command 0x81 returns the value at the pointer.
        if reg == 0x81
            && let Some(p) = self.ene_pointer.get(&addr)
        {
            return Ok(*self.ene.get(&(addr, *p)).unwrap_or(&0));
        }
        Ok(*self.regs.get(&(addr, reg)).unwrap_or(&0))
    }
    fn read_word_data(&mut self, addr: u8, reg: u8) -> Result<u16> {
        self.check(addr)?;
        self.ops.push(Op::ReadWordData(addr, reg));
        if let Some(w) = self.words.get(&(addr, reg)) {
            return Ok(*w);
        }
        Ok((*self.regs.get(&(addr, reg)).unwrap_or(&0) as u16) << 8)
    }
    fn write_byte_data(&mut self, addr: u8, reg: u8, val: u8) -> Result<()> {
        self.check(addr)?;
        self.ops.push(Op::WriteByteData(addr, reg, val));
        if reg == 0x01
            && let Some(p) = self.ene_pointer.get_mut(&addr)
        {
            self.ene.insert((addr, *p), val);
            *p = p.wrapping_add(1);
            return Ok(());
        }
        self.regs.insert((addr, reg), val);
        Ok(())
    }
    fn write_word_data(&mut self, addr: u8, reg: u8, val: u16) -> Result<()> {
        self.check(addr)?;
        self.ops.push(Op::WriteWordData(addr, reg, val));
        if reg == 0x00 {
            // ENE pointer write: the 16-bit register address, byte-swapped.
            self.ene_pointer.insert(addr, val.swap_bytes());
        }
        self.words.insert((addr, reg), val);
        Ok(())
    }
    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
        self.check(addr)?;
        self.ops.push(Op::WriteBlockData(addr, reg, data.to_vec()));
        if self.reject_blocks {
            return Err(Error::Smbus("block write not supported".into()));
        }
        if reg == 0x03
            && let Some(p) = self.ene_pointer.get_mut(&addr)
        {
            for b in data {
                self.ene.insert((addr, *p), *b);
                *p = p.wrapping_add(1);
            }
        }
        Ok(())
    }
}
