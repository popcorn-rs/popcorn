use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::result;
use std::fmt;
use crossbeam::sync::MsQueue;
use spin;

#[derive(Debug, Clone)]
pub enum Error {
  Custom(String)
}

pub type Result = result::Result<(), Error>;
pub type WorkFn = Box<Fn(Result) + Send + 'static>;

/// Enum for inputs into a worker thread
///   Run - contains a function to execute on the worker pool
///   Close - shutdown the worker
pub enum WorkItem {
  Run(WorkFn, Result),
  Close
}

/// Represents an event that can have attached callbacks.
///
/// When an event is triggered, its list of callbacks
/// will be sent to the associated `WorkerPool` for execution.
#[derive(Clone)]
pub struct Event {
  /// Reference to inner data guarded by a spin lock
  inner: Arc<spin::Mutex<EventInner>>
}

/// Inner data for an event. This serves to functions:
///   1. Provide a shareable data structure that can be
///      referenced buy multiple threads.
///   2. Provide a spin lock, which is a desirable lock
///      because all transformations and accesses take
///      only a few operations.
struct EventInner {
  /// Unique id for this event in the `WorkerPool`
  id: usize,

  /// Whether or not this event has completed
  completed: bool,

  /// `WorkerPool` for executing callbacks
  worker_pool: WorkerPool,

  /// List of callbacks to execute when this event triggers
  callbacks: Vec<WorkFn>
}

/// Worker pool of threads that can perform work.
/// Communication occurs through a Michael-Scott
/// queue for maximum efficiency.
#[derive(Clone)]
pub struct WorkerPool {
  /// Inner queue and worker threads
  inner: Arc<WorkerPoolInner>
}

struct WorkerPoolInner {
  /// Current `Event` id
  current_event_id: AtomicUsize,

  /// Michael-Scott queue for sending work items to the threads
  queue: Arc<MsQueue<WorkItem>>,

  /// `JoinHandle`s for all of the worker threads
  workers: Vec<JoinHandle<()>>
}

impl Event {
  /// Get the id of this event.
  ///
  /// ID's are unique to the `WorkerPool` on
  /// which the event was created.
  pub fn id(&self) -> usize {
    let guard = self.inner.lock();
    guard.id
  }

  /// Register a callback for when this event completes.
  ///
  /// If the event is already completed, then this
  /// callback will be immediately queued on the `WorkerPool`
  /// of this event.
  ///
  /// If the event has not yet fired, then it will be queued
  /// on this event's list of callbacks. When the event is
  /// completed with a call to `complete`, then all of the
  /// callbacks will be queued on the `WorkerPool`.
  ///
  /// There are no guarantees about ordering of event callbacks.
  /// The only guarantee is that a callback will be queued onto
  /// the `WorkerPool` and processed at some later point in time.
  pub fn callback<F>(&self, callback: F)
    where F: Fn(Result) + Send + 'static {
      self.callback_box(Box::new(callback))
    }

  pub fn callback_box(&self, f: WorkFn) {
      // Acquire lock to inner data
      let mut guard = self.inner.lock();

      if guard.completed {
        // This event has already completed.
        // Immediately queue the callback.
        guard.worker_pool.spawn_box_fn(f, Ok(()))
      } else {
        // This event has not completed.
        // Queue the callback on the event
        guard.callbacks.push(f)
      }
  }

  /// Complete this event.
  ///
  /// If the event has already been completed,
  /// this operation does nothing and returns false.
  pub fn complete(&self, result: Result) -> bool {
    let mut guard = self.inner.lock();

    if guard.completed {
      false
    } else {
      // Mark the event as completed
      guard.completed = true;

      let len = guard.callbacks.len();
      let worker_pool = guard.worker_pool.clone();

      // Queue all callbacks on the WorkerPool
      for callback in guard.callbacks.drain(0..len) {
        worker_pool.spawn_box_fn(callback, result.clone())
      }

      true
    }
  }
}

impl fmt::Debug for WorkerPool {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "WorkerPool")
  }
}

impl WorkerPool {
  /// Create a new worker pool with a number of
  /// worker threads started and processing.
  pub fn new_with_size(pool_size: usize) -> WorkerPool {
    // Create the Michael-Scott queue to be shared by all workers
    let queue = Arc::new(MsQueue::new());

    // Create all of the worker threads
    let workers: Vec<JoinHandle<()>> = (0..pool_size).map(|i| {
      let q = queue.clone();

      // Build the worker thread
      thread::Builder::new().
        // Create a nice thread name
        name(format!("WorkerThread-{}", i)).
        spawn(move || {
          loop {
            // This will block until a work item is ready
            match q.pop() {
              WorkItem::Run(f, r) => {
                f(r)
              },
              WorkItem::Close => { break; }
            }
          }
        }).unwrap()
    }).collect();

    WorkerPool {
      inner: Arc::new(WorkerPoolInner {
        current_event_id: AtomicUsize::new(0),
        queue: queue,
        workers: workers
      })
    }
  }

  /// Execute a function on a thread in the worker pool.
  pub fn spawn_box_fn(&self,
                      f: WorkFn,
                      r: Result) {
    self.inner.queue.push(WorkItem::Run(f, r))
  }

  /// Create an `Event` driven by this `WorkerPool`
  pub fn create_event(&self) -> Event {
    // Fetch and increment the current_event_id
    let id = self.inner.current_event_id.fetch_add(1, Ordering::SeqCst);

    Event {
      inner: Arc::new(spin::Mutex::new(EventInner {
        id: id,
        completed: false,
        worker_pool: self.clone(),
        callbacks: Vec::new()
      }))
    }
  }
}

/// Release all worker threads, do not wait to join.
impl Drop for WorkerPoolInner {
  fn drop(&mut self) {
    let len = self.workers.len();

    // Send each worker thread a Close message
    for _ in 0..len {
      self.queue.push(WorkItem::Close)
    }
  }
}
