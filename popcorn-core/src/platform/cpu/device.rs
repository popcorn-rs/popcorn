use std::sync::Arc;
use std::ops::Deref;
use uuid::Uuid;
use device::Device;
use super::work::WorkerPool;
use super::event::CpuEvent;

/// Organizes events and callbacks on a CPU. Event
/// callbacks and operations are performed using
/// a `futures` CPUPool to spawn the functions.
#[derive(Debug, Clone)]
pub struct CpuDevice {
  inner: Arc<Inner>
}

#[derive(Debug)]
pub struct Inner {
  /// Unique identifier for this device
  uid: Uuid,

  /// WorkerPool for handling callbacks
  worker_pool: WorkerPool
}

impl CpuDevice {
  /// Create an event that can be triggered later.
  pub fn create_event(&self) -> CpuEvent {
    let work_event = self.inner.worker_pool.create_event();

    CpuEvent::new(self.clone(), work_event)
  }

  /// Get the `WorkerPool` associated with this device
  pub fn worker_pool(&self) -> &WorkerPool { &self.worker_pool }
}

impl Device for CpuDevice {
  fn device_id(&self) -> Uuid {
    self.uid.clone()
  }
}

impl Deref for CpuDevice {
  type Target = Inner;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
