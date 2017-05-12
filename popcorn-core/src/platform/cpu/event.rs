use std::sync::Arc;
use std::fmt;
use super::device::CpuDevice;
use device::DeviceRef;
use event::Event;
use spin;

/// An event that is occurring on a `CpuDevice`.
#[derive(Debug, Clone)]
pub struct CpuEvent {
  inner: Arc<spin::Mutex<Inner>>
}

struct Inner {
  /// Unique identifier on CPU device
  id: usize,

  /// Whether or not this event has been completed
  complete: bool,

  /// CpuDevice responsible for this event
  device: CpuDevice,

  /// List of callbacks for when this event triggers
  callbacks: Vec<Box<Fn() + Send + 'static>>
}

impl CpuEvent {
  pub fn new(id: usize,
             device: CpuDevice) -> CpuEvent {
    CpuEvent {
      inner: Arc::new(spin::Mutex::new(Inner {
        id: id,
        complete: false,
        device: device,
        callbacks: Vec::new()
      }))
    }
  }

  pub fn complete(&self) {
    // Acquire lock on our data
    let mut guard = self.inner.lock();

    // Check if this event is already complete
    if !guard.complete {
      guard.complete = true;
      let len = guard.callbacks.len();
      let pool = guard.device.worker_pool().clone();
      for cb in guard.callbacks.drain(0..len) {
        pool.spawn_box_fn(cb)
      }
    }
  }

  pub fn register_callback<F>(&self, f: F)
    where F: Fn() + Send + 'static {
      // Acquire guard lock
      let mut guard = self.inner.lock();

      // Check if this event is already complete
      if guard.complete {
        // Trigger callback immediately
        guard.device.worker_pool().spawn_box_fn(Box::new(f))
      } else {
        // Queue callback for later
        guard.callbacks.push(Box::new(f))
      }
    }
}

impl Event for CpuEvent {
  fn event_id(&self) -> usize {
    let guard = self.inner.lock();
    guard.id
  }

  fn device(&self) -> DeviceRef {
    let guard = self.inner.lock();
    guard.device.clone().into()
  }
}

impl fmt::Debug for Inner {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Inner {{ id: {}, device: {:?} }}", self.id, self.device)
  }
}
