use futures::{Future, IntoFuture};
use std::marker::PhantomData;
use std::mem;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use device::Device;
use vault::{Vault, VaultAcquired};

use frameworks::native;

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

#[cfg(feature = "native")]
impl From<native::Memory> for BufferMemory {
  fn from(mem: native::Memory) -> BufferMemory { BufferMemory::Native(mem) }
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

  InvalidLock,
  InvalidRawBuffer,
  InvalidDevice,
  InvalidBroadcast
}

#[cfg(feature = "native")]
impl From<native::Error> for Error {
  fn from(err: native::Error) -> Error { Error::Native(err) }
}

#[derive(Debug)]
pub struct LockedBuffer<T> {
  raw: VaultAcquired<RawBuffer<T>>
}

#[derive(Debug, Clone)]
pub struct Buffer<T> {
  raw: Vault<RawBuffer<T>>
}

#[derive(Debug)]
pub struct RawBuffer<T> {
  size: usize,
  copies: HashMap<BufferDevice, BufferMemory>,
  latest_device: BufferDevice,

  _pd: PhantomData<T>,
}

impl<T> Deref for LockedBuffer<T> {
  type Target = RawBuffer<T>;

  fn deref(&self) -> &RawBuffer<T> { &self.raw }
}

impl<T> DerefMut for LockedBuffer<T> {
  fn deref_mut(&mut self) -> &mut RawBuffer<T> { &mut self.raw }
}

impl<T: Send + Copy + Sized + 'static> RawBuffer<T> {
  pub fn new<D: Into<BufferDevice>>(dev: D, size: usize) -> Result<RawBuffer<T>, Error> {
    let bdev: BufferDevice = dev.into();
    let mut copies = HashMap::new();
    let copy = try!(Self::alloc_on_device(&bdev, size * mem::size_of::<T>()));
    copies.insert(bdev.clone(), copy);

    Ok(RawBuffer {
      size: size,
      copies: copies,
      latest_device: bdev,
      _pd: PhantomData
    })
  }

  #[cfg(feature = "native")]
  pub fn from_vec_native(dev: &native::Device, vec: Vec<T>) -> Result<RawBuffer<T>, Error> {
    let bdev: BufferDevice = dev.into();
    let mut copies = HashMap::new();
    let mut mem = try!(dev.alloc_memory(vec.len() * mem::size_of::<T>()));
    try!(mem.copy_from(&vec));
    let copy = BufferMemory::Native(mem);
    copies.insert(bdev.clone(), copy);

    Ok(RawBuffer {
      size: vec.len(),
      copies: copies,
      latest_device: bdev,
      _pd: PhantomData
    })
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

  pub fn size(&self) -> usize {
    self.size
  }
}

impl<T: Send + Copy + Sized + 'static> LockedBuffer<T> {
  pub fn sync_from_vec(mut self, vec: Vec<T>) -> Box<Future<Item=LockedBuffer<T>,Error=Error>> {
    let dev = self.latest_device.clone();
    let copy = self.copies.remove(&dev);

    match copy {
      Some(mem) => {
        match dev {
          #[cfg(feature = "native")]
          BufferDevice::Native(ref dev) => {
            let BufferMemory::Native(m) = mem;
            let new_dev = BufferDevice::Native(dev.clone());
            Box::new(dev.sync_from_vec(m, vec).map(move |mem| {
              self.raw.copies.insert(new_dev, BufferMemory::Native(mem));
              self
            }).map_err(Error::Native))
          },
        }
      },
      None => Box::new(Err(Error::InvalidDevice).into_future())
    }
  }

  pub fn sync_to_vec(mut self) -> Box<Future<Item=Vec<T>,Error=Error>> {
    let dev = self.latest_device.clone();
    let copy = self.copies.remove(&dev);
    match copy {
      Some(mem) => {
        match dev {
          #[cfg(feature = "native")]
          BufferDevice::Native(ref dev) => {
            let BufferMemory::Native(m) = mem;
            let new_dev = BufferDevice::Native(dev.clone());
            Box::new(dev.sync_to_vec(m).map(move |(mem, vec)| {
              self.copies.insert(new_dev, BufferMemory::Native(mem));
              vec
            }).map_err(Error::Native))
          },
        }
      },
      None => Box::new(Err(Error::InvalidDevice).into_future())
    }
  }

  pub fn sync<D: Into<BufferDevice>>(self, dev: D) -> Box<Future<Item=LockedBuffer<T>,Error=Error>> {
    let bdev = dev.into();

    match self.latest_device {
      #[cfg(feature = "native")]
      BufferDevice::Native(_) => {
        match bdev {
          #[cfg(feature = "native")]
          BufferDevice::Native(_) => Box::new(Ok(self).into_future()),
        }
      },
    }
  }
}

impl<T> From<RawBuffer<T>> for Buffer<T> {
  fn from(raw: RawBuffer<T>) -> Buffer<T> {
    let vault_raw = Vault::new(raw);

    Buffer {
      raw: vault_raw
    }
  }
}

impl<T> From<LockedBuffer<T>> for Buffer<T> {
  fn from(locked: LockedBuffer<T>) -> Buffer<T> {
    Buffer {
      raw: locked.raw.into()
    }
  }
}

impl<T> From<VaultAcquired<RawBuffer<T>>> for LockedBuffer<T> {
  fn from(locked: VaultAcquired<RawBuffer<T>>) -> LockedBuffer<T> {
    LockedBuffer {
      raw: locked
    }
  }
}

impl<T: Send + Copy + Sized + 'static> Buffer<T> {
  pub fn new<D: Into<BufferDevice>>(dev: D, size: usize) -> Result<Buffer<T>, Error> {
    let raw = try!(RawBuffer::new(dev, size));
    Ok(raw.into())
  }

  #[cfg(feature = "native")]
  pub fn with_capacity_native(dev: &native::Device, capacity: usize) -> Result<Buffer<T>, Error> {
    Self::new(dev, capacity)
  }

  #[cfg(feature = "native")]
  pub fn from_vec_native(dev: &native::Device, vec: Vec<T>) -> Result<Buffer<T>, Error> {
    let raw = try!(RawBuffer::from_vec_native(dev, vec));
    Ok(raw.into())
  }

  pub fn lock(&self) -> Box<Future<Item=LockedBuffer<T>,Error=Error>> {
    Box::new(self.raw.lock().map(|raw| LockedBuffer {
      raw: raw
    }).map_err(|_| Error::InvalidLock))
  }

  pub fn try_lock(&self) -> Result<LockedBuffer<T>,Error> {
    self.raw.try_lock().map(|raw| raw.into()).map_err(|_| Error::InvalidLock)
  }
}
