use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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

  /// Current event id
  current_event_id: AtomicUsize,

  /// WorkerPool for handling callbacks
  worker_pool: WorkerPool
}

impl CpuDevice {
  pub fn create_event(&self) -> CpuEvent {
    let id = self.current_event_id.fetch_add(1, Ordering::SeqCst);

    CpuEvent::new(id, self.clone())
  }

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
