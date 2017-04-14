use futures::{Future, IntoFuture};
use std::marker::PhantomData;
use std::mem;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, LockResult};
use device::Device;

use frameworks::native;

#[derive(Debug, Clone, Copy)]
pub enum BufferSource {
  #[cfg(feature = "native")]
  Native
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum BufferDevice {
  #[cfg(feature = "native")]
  Native(native::Device)
}

#[cfg(feature = "native")]
impl From<native::Device> for BufferDevice {
  fn from(dev: native::Device) -> BufferDevice { BufferDevice::Native(dev) }
}

#[cfg(feature = "native")]
impl<'a> From<&'a native::Device> for BufferDevice {
  fn from(dev: &'a native::Device) -> BufferDevice { BufferDevice::Native(dev.clone()) }
}

#[derive(Debug)]
pub enum BufferMemory {
  #[cfg(feature = "native")]
  Native(native::Memory)
}

#[derive(Debug, Clone)]
pub enum Error {
  #[cfg(feature = "native")]
  Native(native::Error),

  InvalidRawBuffer,
  InvalidDevice,
  InvalidBroadcast
}

#[cfg(feature = "native")]
impl From<native::Error> for Error {
  fn from(err: native::Error) -> Error { Error::Native(err) }
}

#[derive(Debug)]
pub struct Buffer<T: Copy + Sized + Send + 'static> {
  raw: Arc<RwLock<RawBuffer<T>>>
}

#[derive(Debug)]
pub struct RawBuffer<T: Copy + Sized + Send + 'static> {
  size: usize,
  copies: HashMap<BufferDevice, BufferMemory>,
  latest_source: BufferSource,

  _pd: PhantomData<T>,
}

impl<T: Send + Copy + Sized + 'static> RawBuffer<T> {
  pub fn new<D: Into<BufferDevice>>(dev: D, size: usize) -> Result<RawBuffer<T>, Error> {
    let bdev: BufferDevice = dev.into();
    let mut copies = HashMap::new();
    let latest_source = Self::device_source(&bdev);
    let copy = try!(Self::alloc_on_device(&bdev, size * mem::size_of::<T>()));
    copies.insert(bdev, copy);

    Ok(RawBuffer {
      size: size,
      copies: copies,
      latest_source: latest_source,
      _pd: PhantomData
    })
  }

  pub fn source(&self) -> BufferSource { self.latest_source }

  pub fn device_source(dev: &BufferDevice) -> BufferSource {
    match *dev {
      #[cfg(feature = "native")]
      BufferDevice::Native(_) => BufferSource::Native,
    }
  }

  fn alloc_on_device(dev: &BufferDevice, size: usize) -> Result<BufferMemory, Error> {
    match *dev {
      #[cfg(feature = "native")]
      BufferDevice::Native(ref dev_n) => Self::alloc_on_device_native(dev_n, size),
    }
  }

  fn alloc_on_device_native(dev: &native::Device, size: usize) -> Result<BufferMemory, Error> {
    dev.alloc_memory(size).
      map(|m| BufferMemory::Native(m)).
      map_err(|e| Error::Native(e))
  }

  #[cfg(feature = "native")]
  pub fn native_memory(&self, dev: &native::Device) -> Result<&native::Memory, Error> {
    match self.copies.get(&BufferDevice::Native(dev.clone())) {
      Some(mem) => {
        let BufferMemory::Native(ref nm) = *mem;
        Ok(nm)
      },
      None => Err(Error::InvalidDevice)
    }
  }

  #[cfg(feature = "native")]
  pub fn native_memory_mut(&mut self, dev: &native::Device) -> Result<&mut native::Memory, Error> {
    match self.copies.get_mut(&BufferDevice::Native(dev.clone())) {
      Some(mem) => {
        let BufferMemory::Native(ref mut nm) = *mem;
        Ok(nm)
      },
      None => Err(Error::InvalidDevice)
    }
  }
}

impl<T: Send + Copy + Sized + 'static> From<RawBuffer<T>> for Buffer<T> {
  fn from(raw: RawBuffer<T>) -> Buffer<T> {
    let arc = Arc::new(RwLock::new(raw));
    Buffer {
      raw: arc
    }
  }
}

impl<T: Send + Copy + Sized + 'static> From<Arc<RwLock<RawBuffer<T>>>> for Buffer<T> {
  fn from(raw: Arc<RwLock<RawBuffer<T>>>) -> Buffer<T> {
    Buffer {
      raw: raw
    }
  }
}

impl<T: Send + Copy + Sized + 'static> Buffer<T> {
  pub fn new<D: Into<BufferDevice>>(dev: D, size: usize) -> Result<Buffer<T>, Error> {
    let raw = try!(RawBuffer::new(dev, size));
    Ok(raw.into())
  }

  pub fn read<'a>(&'a self) -> LockResult<RwLockReadGuard<'a, RawBuffer<T>>> {
    self.raw.read()
  }

  pub fn write<'a>(&'a mut self) -> LockResult<RwLockWriteGuard<'a, RawBuffer<T>>> {
    self.raw.write()
  }

  pub fn sync_from_vec<D: Into<BufferDevice>>(self, vec: Vec<T>, dev: D) -> Box<Future<Item=Buffer<T>,Error=Error>> {
    let bdev: BufferDevice = dev.into();
    let copy = {
      let mut raw = self.raw.write().unwrap();
      raw.copies.remove(&bdev)
    };

    match copy {
      Some(mem) => {
        match bdev {
          #[cfg(feature = "native")]
          BufferDevice::Native(ref dev) => {
            let BufferMemory::Native(m) = mem;
            let new_dev = BufferDevice::Native(dev.clone());
            Box::new(dev.sync_from_vec(m, vec).map(move |mem| {
              {
                let mut raw = self.raw.write().unwrap();
                raw.latest_source = RawBuffer::<T>::device_source(&new_dev);
                raw.copies.insert(new_dev, BufferMemory::Native(mem));
              }
              self
            }).map_err(Error::Native))
          },
        }
      },
      None => Box::new(Err(Error::InvalidDevice).into_future())
    }
  }

  pub fn sync_to_vec<D: Into<BufferDevice>>(self, dev: D) -> Box<Future<Item=(Buffer<T>, Vec<T>),Error=Error>> {
    let bdev: BufferDevice = dev.into();
    let copy = {
      let mut raw = self.raw.write().unwrap();
      raw.copies.remove(&bdev)
    };
    match copy {
      Some(mem) => {
        match bdev {
          #[cfg(feature = "native")]
          BufferDevice::Native(ref dev) => {
            let BufferMemory::Native(m) = mem;
            let new_dev = BufferDevice::Native(dev.clone());
            Box::new(dev.sync_to_vec(m).map(move |(mem, vec)| {
              {
                let mut raw = self.raw.write().unwrap();
                raw.copies.insert(new_dev, BufferMemory::Native(mem));
              }

              (self, vec)
            }).map_err(Error::Native))
          },
        }
      },
      None => Box::new(Err(Error::InvalidDevice).into_future())
    }
  }

  pub fn sync(self, dev: &BufferDevice) -> Box<Future<Item=Buffer<T>,Error=Error>> {
    let source = {
      let raw = self.raw.read().unwrap();
      raw.source()
    };
    match source {
      #[cfg(feature = "native")]
      BufferSource::Native => {
        match *dev {
          #[cfg(feature = "native")]
          BufferDevice::Native(_) => Box::new(Ok(self).into_future()),
        }
      },
    }
  }
}
