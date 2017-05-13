use std::sync::Arc;
use std::fmt;
use std::cell::Cell;
use super::device::CpuDevice;
use super::work;
use device::DeviceRef;
use event::{self, Event};
use spin;

/// An event that is occurring on a `CpuDevice`.
#[derive(Debug, Clone)]
pub struct CpuEvent {
  inner: Arc<Inner>
}

struct Inner {
  /// Device for this event
  device: CpuDevice,

  /// Underlying event for the `WorkerPool`
  work_event: work::Event,

  /// Result of this event
  result: spin::Mutex<Cell<event::Result>>
}

impl CpuEvent {
  /// Create a CpuEvent with a device and a `work::Event`.
  pub fn new(device: CpuDevice,
             work_event: work::Event) -> CpuEvent {
    CpuEvent {
      inner: Arc::new(Inner {
        device: device,
        work_event: work_event,
        result: spin::Mutex::new(Cell::new(Err(event::Error::NotCompleted)))
      })
    }
  }
}

impl Event for CpuEvent {
  fn event_id(&self) -> usize { self.inner.work_event.id() }

  fn device(&self) -> DeviceRef {
    self.inner.device.clone().into()
  }

  fn callback(&self, f: event::CallbackFn) {
    let event = self.clone();
    let callback = Box::new(move || {
      f(Box::new(event.clone()))
    }) as work::WorkFn;

    self.inner.work_event.callback_box(callback)
  }

  fn complete(&self, result: event::Result) {
    let mut guard = self.inner.result.lock();
    guard.set(result);
    self.inner.work_event.complete();
  }

  fn result(&self) -> event::Result {
    let guard = self.inner.result.lock();
    guard.get().clone()
  }
}

impl fmt::Debug for Inner {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Inner {{ id: {}, device: {:?} }}", self.work_event.id(), self.device)
  }
}
