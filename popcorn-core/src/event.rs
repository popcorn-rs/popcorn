use std::result;
use device::DeviceRef;

#[derive(Debug, Clone, Copy)]
pub enum Error {
  NotCompleted
}

pub type Result = result::Result<(), Error>;
pub type CallbackFn = Box<Fn(Box<Event + Send + 'static>) + Send + 'static>;

/// Trait for events occurring on a device.
/// All events have a unique `usize` identifier and
/// a `Device` associated with them for handling
/// callbacks and event queueing.
pub trait Event {
  /// Device-unique identifier for this event
  fn event_id(&self) -> usize;

  /// Device associated with this event
  fn device(&self) -> DeviceRef;

  /// Register a callback without an event.
  fn callback(&self, f: CallbackFn);

  /// Complete this event.
  fn complete(&self, result: Result);

  /// Get the result
  fn result(&self) -> Result;
}
