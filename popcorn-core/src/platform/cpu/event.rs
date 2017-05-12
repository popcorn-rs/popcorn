use std::sync::Arc;
use std::fmt;
use super::device::CpuDevice;
use super::work;
use device::DeviceRef;
use event::Event;

/// An event that is occurring on a `CpuDevice`.
#[derive(Debug, Clone)]
pub struct CpuEvent {
  inner: Arc<Inner>
}

struct Inner {
  /// Device for this event
  device: CpuDevice,

  /// Underlying event for the `WorkerPool`
  work_event: work::Event
}

impl CpuEvent {
  pub fn new(device: CpuDevice,
             work_event: work::Event) -> CpuEvent {
    CpuEvent {
      inner: Arc::new(Inner {
        device: device,
        work_event: work_event
      })
    }
  }
}

impl Event for CpuEvent {
  fn event_id(&self) -> usize { self.inner.work_event.id() }

  fn device(&self) -> DeviceRef {
    self.inner.device.clone().into()
  }

  fn event_callback(&self, f: Box<Fn() + Send + 'static>) -> Box<Event> {
    let event = self.inner.device.create_event();
    let box_event = Box::new(event.clone()) as Box<Event>;

    self.inner.work_event.callback(move || {
      f();
      event.inner.work_event.complete();
    });

    box_event
  }

  fn callback(&self, f: Box<Fn() + Send + 'static>) {
    self.inner.work_event.callback_box(f)
  }
}

impl fmt::Debug for Inner {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Inner {{ id: {}, device: {:?} }}", self.work_event.id(), self.device)
  }
}
