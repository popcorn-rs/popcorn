//! Popcorn provides a simple, unified framework for executing
//! non-blocking operations in parallel across multiple devices.
//!
//! The initial targeted devices are CPU, CUDA/HIP and Vulkan to
//! allow for maximum speed and portability. The [Futures-rs](https://github.com/alexcrichton/futures-rs)
//! crate is used to execute operations in the correct order while
//! still allowing for operations to execute in a parallel and
//! non-blocking fashion.
//!
//! # Abstract
//!
//! # Project Goals
//!
//! 1. Run on any device making optimal use of hardware.
//! 2. Execute extremely fast.
//! 3. Take up as little memory as possible.
//! 4. Support cutting-edge AI/ML algorithms.
//!
//! # Design

extern crate futures;
extern crate futures_cpupool;
extern crate spin;

pub mod backend;
pub mod hardware;
pub mod framework;
pub mod memory;
pub mod device;
pub mod buffer;
pub mod frameworks;
pub mod vault;

pub use backend::Backend;
pub use hardware::Hardware;
pub use framework::Framework;
pub use memory::Memory;
pub use device::Device;
pub use buffer::{LockedBuffer, Buffer, BufferDevice};
pub use vault::Vault;

pub use frameworks::native;

#[cfg(test)]
mod test {
  use super::*;
  use futures::Future;

  #[test]
  #[cfg(feature = "native")]
  fn test_native() {
    let backend = native::Backend::default();
    let dev = backend.device();
    let buf: Buffer<f64> = Buffer::new(dev, 4).unwrap();
    let lbuf = buf.lock();
    let f1 = lbuf.and_then(|b| b.sync_from_vec(vec![23.0, 45.5, 54.2, 42.0]));
    let f2 = f1.and_then(|x| x.sync_to_vec());
    let nv = f2.wait().unwrap();

    assert_eq!(nv, vec![23.0, 45.5, 54.2, 42.0]);
  }
}
