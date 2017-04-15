use std::sync::Arc;
use std::cell::UnsafeCell;
use std::ops::{Drop, Deref, DerefMut};
use futures::{Future, Poll, Async};
use futures::task::{self, Task};
use spin;

#[derive(Debug)]
pub struct Vault<T> {
  inner: Arc<Inner<T>>
}

#[derive(Debug)]
pub struct InnerLock {
  locked: bool,
  waiting: Vec<Task>,
}

#[derive(Debug)]
pub struct Inner<T> {
  lock: spin::Mutex<InnerLock>,
  data: UnsafeCell<T>
}

unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send> Sync for Inner<T> {}

#[derive(Debug)]
pub struct VaultAcquire<T> {
  inner: Arc<Inner<T>>
}

#[derive(Debug)]
pub struct VaultAcquired<T> {
  inner: Arc<Inner<T>>
}

impl<T> Vault<T> {
  pub fn new(t: T) -> Vault<T> {
    let inner = Arc::new(Inner {
      lock: spin::Mutex::new(InnerLock {
        locked: false,
        waiting: Vec::with_capacity(2),
      }),
      data: UnsafeCell::new(t)
    });

    Vault {
      inner: inner
    }
  }

  pub fn acquire(&self) -> VaultAcquire<T> {
    VaultAcquire {
      inner: self.inner.clone()
    }
  }
}

impl<T> From<T> for Vault<T> {
  fn from(t: T) -> Vault<T> {
    Vault::new(t)
  }
}

impl<T> Future for VaultAcquire<T> {
  type Item = VaultAcquired<T>;
  type Error = ();

  fn poll(&mut self) -> Poll<VaultAcquired<T>, ()> {
    let mut lock = self.inner.lock.lock();

    if !lock.locked {
      lock.locked = true;
      Ok(VaultAcquired {
        inner: self.inner.clone()
      }.into())
    } else {
      let task = task::park();
      lock.waiting.push(task);
      Ok(Async::NotReady)
    }
  }
}

impl<T> Deref for VaultAcquired<T> {
  type Target = T;

  fn deref(&self) -> &T { unsafe { &*self.inner.data.get() } }
}

impl<T> DerefMut for VaultAcquired<T> {
  fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.inner.data.get() } }
}

impl<T> Drop for VaultAcquired<T> {
  fn drop(&mut self) {
    let mut lock = self.inner.lock.lock();
    assert!(lock.locked);

    lock.locked = false;
    match lock.waiting.pop() {
      Some(task) => task.unpark(),
      None => { }
    }
  }
}

