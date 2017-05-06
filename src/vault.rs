//! Provides a synchronization primitive for a shared
//! data structure which can be acquired and released at a later time.
//! Unlike an `std::sync::RwLock`, the guard does not have
//! an explicit lifetime associated with the locked data. This means
//! that the lock can be acquired and maintained across futures operations.
//!
//! This is useful for sharing data across operations that require multiple
//! futures to fully complete. For instance, in `popcorn`, a common pattern
//! is to first synchronize a `Buffer` to a device with one future and then
//! perform an operation on the `Buffer` with another future. Acquiring a lock
//! separately in each future does not offer strong enough guarantees in this
//! scenario, as another operation may be trying to sync the `Buffer` to
//! another device simultaneously. The `Vault` allows us to lock the `Buffer`
//! for the entire sequence of futures required to execute an operation.

use std::sync::Arc;
use std::cell::UnsafeCell;
use std::ops::{Drop, Deref, DerefMut};
use std::clone::Clone;
use futures::{Future, Poll, Async};
use futures::task::{self, Task};
use spin;

/// A `Vault` holds shared data that can be acquired across multiple threads
/// of execution. Once acquired, the underlying data is locked until the
/// `VaultAcquired` object is dropped. The `Vault` may be acquired in one
/// thread and released in another at a future point in time. This allows
/// us to lock a data structure for an entire series of future transformations
/// that are required to execute without being interrupted but are designed
/// to be executed in a non-blocking manner.
///
/// Acquiring a lock is entirely non-blocking and future-driven, similar to
/// a `BiLock` from the `futures-rs` crate.
#[derive(Debug)]
pub struct Vault<T> {
  inner: Arc<Inner<T>>
}

/// The `InnerLock` structure holds data signaling if the data is currently
/// locked or not and which tasks are waiting to acquire the lock.
///
/// When attempting to lock a `Vault` which is not yet acquired, the `locked`
/// fields will be switched to true. If the `Vault` has already been acquired
/// by another owner, then the current thread will park a task which will be
/// unparked at a later time when the lock has been released. Multiple threads
/// can be waiting to acquire a lock at once.
#[derive(Debug)]
pub struct InnerLock {
  /// Whether or not this vault has been acquired
  locked: bool,

  /// A queue of tasks that are waiting to acquire this vault
  waiting: Vec<Task>,
}

/// The `Inner` structure holds a reference to a spin mutex which is used
/// for synchronizing between threads as well as the underlying data cell.
///
/// A spin mutex is very efficient as long as the lock is never acquired for
/// too long. In our case, we only ever acquire the lock to check a boolean
/// and maybe queue ourselves on a waiting list. Both are very quick operations,
/// making a spin lock much more performant than other locking structures.
#[derive(Debug)]
pub struct Inner<T> {
  /// A spin lock that guards access to the inner lock data
  lock: spin::Mutex<InnerLock>,

  /// The actual data stored in the Vault
  data: UnsafeCell<T>
}

// We can send our Vaults across threads
unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send> Sync for Inner<T> {}

/// `VaultAcquire` is returned by calling `lock` on a `Vault`. This structure
/// has not actually locked the `Vault` yet, but it implements the `Future`
/// trait, which will resolve to a `VaultAcquired`, which can be used to
/// access the synchronized data.
#[derive(Debug)]
pub struct VaultAcquire<T> {
  inner: Arc<Inner<T>>
}

/// `VaultAcquired` is the structure that let's us actually access the
/// underlying data of the `Vault`. Once we have this data structure,
/// our data is locked as long as it exists. Once it is dropped, the
/// lock is released and another process can access the data.
#[derive(Debug)]
pub struct VaultAcquired<T> {
  inner: Arc<Inner<T>>
}

impl<T> Clone for Vault<T> {
  fn clone(&self) -> Self {
    Vault {
      inner: self.inner.clone()
    }
  }
}

impl<T> Vault<T> {
  /// Create a new `Vault` containing some data.
  ///
  /// # Example
  ///
  /// ```
  /// # use popcorn::vault::*;
  /// #
  /// // Create a new Vault containing a double
  /// let vault: Vault<f64> = Vault::new(34.5);
  ///
  /// // Acquire the lock to the vault
  /// let acquired = vault.try_lock().unwrap();
  ///
  /// // Access the value in the vault
  /// let value: f64 = *acquired;
  /// #
  /// # assert_eq!(value, 34.5);
  /// ```
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

  /// Lock the `Vault` for future access to its data.
  /// This operation does not block and will return a
  /// `VaultAcquire` object, which is a future that
  /// resolves to a `VaultAcquired` object. Once the future
  /// has resolved, we are able to access the underlying
  /// data of the `Vault`.
  ///
  /// # Example
  ///
  /// ```
  /// # extern crate futures;
  /// # extern crate popcorn;
  /// #
  /// # use popcorn::vault::*;
  /// # use futures::Future;
  /// #
  /// # fn main() {
  /// // Create a new Vault containing a double
  /// let vault: Vault<f64> = Vault::new(42.0);
  ///
  /// // Acquire a future lock on the data
  /// let acquire = vault.lock();
  ///
  /// // Wait until we have locked the data
  /// let acquired = vault.try_lock().unwrap();
  ///
  /// // Access the value in the vault
  /// let value: f64 = *acquired;
  /// #
  /// # assert_eq!(value, 42.0);
  /// # }
  /// ```
  pub fn lock(&self) -> VaultAcquire<T> {
    VaultAcquire {
      inner: self.inner.clone()
    }
  }

  /// Trys to lock the `Vault` immediately. This function
  /// is meant to be used outside of the futures API and
  /// only when it is known that the `Vault` can be locked.
  ///
  /// # Example
  ///
  /// ```
  /// # use popcorn::vault::*;
  /// #
  /// // Create a new Vault containing a double
  /// let vault: Vault<f64> = Vault::new(34.5);
  ///
  /// // Acquire the lock to the vault
  /// let acquired = vault.try_lock().unwrap();
  ///
  /// // Access the value in the vault
  /// let value: f64 = *acquired;
  /// #
  /// # assert_eq!(value, 34.5);
  /// ```
  pub fn try_lock(&self) -> Result<VaultAcquired<T>, ()> {
    let mut lock = self.inner.lock.lock();

    if !lock.locked {
      lock.locked = true;
      Ok(VaultAcquired {
        inner: self.inner.clone()
      })
    } else {
      Err(())
    }
  }
}

/// Move any data type into a `Vault`.
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
  /// Release the lock on the `Vault` when `VaultAcquired` is dropped.
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

impl<T> From<VaultAcquired<T>> for Vault<T> {
  fn from(va: VaultAcquired<T>) -> Vault<T> {
    Vault {
      inner: va.inner.clone()
    }
  }
}

