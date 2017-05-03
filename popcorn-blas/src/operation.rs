use futures::Future;
use popcorn::buffer::{LockedBuffer, Error};

pub trait DotOperation<T: Copy + Send + 'static> {
  fn bcast_dot(&self,
               shape_a: LockedBuffer<usize>,
               a: LockedBuffer<T>,
               shape_b: LockedBuffer<usize>,
               b: LockedBuffer<T>) ->
    Box<Future<Item=(LockedBuffer<usize>, LockedBuffer<T>), Error=Error>>; // Result
}
