use device::DeviceRef;

/// Trait for events occurring on a device.
/// All events have a unique `usize` identifier and
/// a `Device` associated with them for handling
/// callbacks and event queueing.
pub trait Event: Clone {
  /// Device-unique identifier for this event
  fn event_id(&self) -> usize;

  /// Device associated with this event
  fn device(&self) -> DeviceRef;
}
