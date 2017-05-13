use std::sync::Arc;
use std::ops::Deref;
use uuid::Uuid;
use memory::Memory;
use event::Event;

/// A reference to a device
pub struct DeviceRef(Arc<Box<Device>>);

/// Trait for devices that can execute kernels and
/// handle events/callbacks. All devices should be
/// thread-safe.
pub trait Device {
  /// Unique identifier for this device
  fn device_id(&self) -> Uuid;

  /// Create an event on this device
  fn create_event(&self) -> Box<Event>;

  /// Allocate memory on this device
  fn allocate(&self,
              size: usize,
              element_size: usize) -> (Box<Memory>, Box<Event>);
}

/// Convenience conversion from a device into a reference
impl<D: Device + 'static> From<D> for DeviceRef {
  fn from(device: D) -> DeviceRef {
    DeviceRef(Arc::new(Box::new(device) as Box<Device>))
  }
}

/// Convenience deref for accessing internal values
impl Deref for DeviceRef {
  type Target = Arc<Box<Device>>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
