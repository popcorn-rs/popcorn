pub mod broadcast;
pub mod core_ops;

use popcorn::frameworks::native::Framework;
use popcorn::backend::Backend;
use operation::*;
use futures::Future;
use popcorn::buffer::{LockedBuffer, Error};
use std::fmt;

pub use self::core_ops::*;

impl<B: Backend<Framework>, T: Dot + fmt::Debug + Sync + Copy + Sized + Send + 'static> DotOperation<T> for B {
  fn bcast_dot(&self,
               shape_a: LockedBuffer<usize>,
               a: LockedBuffer<T>,
               shape_b: LockedBuffer<usize>,
               b: LockedBuffer<T>,
               shape_c: LockedBuffer<usize>,
               c: LockedBuffer<T>) ->
    Box<Future<Item=(LockedBuffer<usize>, LockedBuffer<T>,
                     LockedBuffer<usize>, LockedBuffer<T>,
                     LockedBuffer<usize>, LockedBuffer<T>), Error=Error>> {
      // Step 1. Sync all input buffers to the required device
      let dev = self.device();
      let ar = shape_a.sync(dev).join(a.sync(dev));
      let br = shape_b.sync(dev).join(b.sync(dev));
      let cr = shape_c.sync(dev).join(c.sync(dev));


      // Step 2. Convert all memory to native memory and execute the
      //   broadcasted dot operation on the cpu pool
      let dev = self.device().clone();
      let pool = self.device().pool().clone();
      Box::new(ar.join(br).join(cr).and_then(move |(((shape_a, a), (shape_b, b)), (mut shape_c, mut c))| {
        pool.spawn_fn(move || {
          {
            let n_shape_a: &[usize] = try!(try!(shape_a.native_memory(&dev)).try_as_slice());
            let n_a: &[T] = try!(try!(a.native_memory(&dev)).try_as_slice());
            let n_shape_b: &[usize] = try!(try!(shape_b.native_memory(&dev)).try_as_slice());
            let n_b: &[T] = try!(try!(b.native_memory(&dev)).try_as_slice());
            let n_shape_c: &mut [usize] = try!(try!(shape_c.native_memory_mut(&dev)).try_as_mut_slice());
            let n_c: &mut [T] = try!(try!(c.native_memory_mut(&dev)).try_as_mut_slice());

            let (mut bshape, iter_a, iter_b) = try!(broadcast::try_new_broadcast(n_shape_a, n_a, n_shape_b, n_b, 1));
            bshape.pop();

            let r_iter = iter_a.zip(iter_b).map(|(a, b)| T::dot(a, b));
            for (v1, v2) in bshape.iter().zip(n_shape_c.iter_mut()) {
              *v2 = *v1;
            }

            for(v1, v2) in r_iter.zip(n_c.iter_mut()) {
              *v2 = v1;
            }
          }

          Ok((shape_a, a,
              shape_b, b,
              shape_c, c))
        })
      }))
    }
}
