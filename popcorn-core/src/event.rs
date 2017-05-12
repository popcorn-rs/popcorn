use std::result;
use device::DeviceRef;

#[derive(Debug, Clone)]
pub enum Error {
  Custom(String)
}

pub type Result = result::Result<(), Error>;
pub type CallbackFn = Box<Fn(Result) -> Result + Send + 'static>;

/// Trait for events occurring on a device.
/// All events have a unique `usize` identifier and
/// a `Device` associated with them for handling
/// callbacks and event queueing.
pub trait Event {
  /// Device-unique identifier for this event
  fn event_id(&self) -> usize;

  /// Device associated with this event
  fn device(&self) -> DeviceRef;

  /// Register a callback with an event for
  /// when it completes.
  fn event_callback(&self, f: CallbackFn) -> Box<Event>;

  /// Register a callback without an event.
  fn callback(&self, f: CallbackFn);
}
