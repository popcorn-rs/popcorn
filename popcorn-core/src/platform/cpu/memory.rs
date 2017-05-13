use memory::Memory;

pub struct CpuMemory { }

impl CpuMemory {
  pub fn new() -> CpuMemory {
    CpuMemory { }
  }
}

impl Memory for CpuMemory { }
