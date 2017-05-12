use std::sync::Arc;
use std::fmt;
use super::device::CpuDevice;
use super::convert;
use super::work;
use device::DeviceRef;
use event::{self, Event};

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
  /// Create a CpuEvent with a device and a `work::Event`.
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

  fn event_callback(&self, f: event::CallbackFn) -> Box<Event> {
    let event = self.inner.device.create_event();
    let box_event = Box::new(event.clone()) as Box<Event>;

    self.inner.work_event.callback(move |r| {
      let nr = convert::event_result_to_work(f(convert::work_result_to_event(r)));
      event.inner.work_event.complete(nr);
    });

    box_event
  }

  fn callback(&self, f: event::CallbackFn) {
    let callback = Box::new(move |r| {
      match f(convert::work_result_to_event(r)) {
        Ok(()) => { },
        Err(_) => { }
      }
    }) as work::WorkFn;

    self.inner.work_event.callback_box(callback)
  }
}

impl fmt::Debug for Inner {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Inner {{ id: {}, device: {:?} }}", self.work_event.id(), self.device)
  }
}
