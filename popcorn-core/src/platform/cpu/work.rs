use std::sync::Arc;
use std::ops::Deref;
use std::thread::{self, JoinHandle};
use crossbeam::sync::MsQueue;
use std::fmt;

/// Enum for inputs into a worker thread
///   Run - contains a function to execute on the worker pool
///   Close - shutdown the worker
pub enum WorkItem {
  Run(Box<Fn() -> () + Send>),
  Close
}

/// Worker pool of threads that can perform work.
/// Communication occurs through a Michael-Scott
/// queue for maximum efficiency.
#[derive(Clone)]
pub struct WorkerPool {
  /// Inner queue and worker threads
  inner: Arc<Inner>
}

struct Inner {
  /// Michael-Scott queue for sending work items to the threads
  queue: Arc<MsQueue<WorkItem>>,

  /// `JoinHandle`s for all of the worker threads
  workers: Vec<JoinHandle<()>>
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
              WorkItem::Run(f) => {
                f()
              },
              WorkItem::Close => { break; }
            }
          }
        }).unwrap()
    }).collect();

    WorkerPool {
      inner: Arc::new(Inner {
        queue: queue,
        workers: workers
      })
    }
  }

  /// Execute a function on a thread in the worker pool.
  pub fn spawn_box_fn(&self, f: Box<Fn() + Send + 'static>) {
    self.inner.queue.push(WorkItem::Run(f))
  }
}

/// Release all worker threads, do not wait to join.
impl Drop for Inner {
  fn drop(&mut self) {
    let len = self.workers.len();

    // Send each worker thread a Close message
    for _ in 0..len {
      self.queue.push(WorkItem::Close)
    }
  }
}
