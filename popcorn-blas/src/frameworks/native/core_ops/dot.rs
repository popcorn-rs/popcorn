use super::super::broadcast;
use popcorn::frameworks::native::Framework;
use popcorn::backend::Backend;
use operation::*;
use futures::Future;
use popcorn::buffer::{Buffer, LockedBuffer, Error};
use std::fmt;
use blas_sys::c::cblas_sdot;

pub trait Dot where Self: Sized {
  fn dot(a: &[Self], b: &[Self]) -> Self;
}

impl Dot for f32 {
  fn dot(a: &[Self], b: &[Self]) -> Self {
    unsafe {
      cblas_sdot(a.len() as i32, a.as_ptr(), 1, b.as_ptr(), 1)
    }
  }
}

impl<B: Backend<Framework>, T: Dot + fmt::Debug + Sync + Copy + Sized + Send + 'static> DotOperation<T> for B {
  fn bcast_dot(&self,
               shape_a: LockedBuffer<usize>,
               a: LockedBuffer<T>,
               shape_b: LockedBuffer<usize>,
               b: LockedBuffer<T>) ->
    Box<Future<Item=(LockedBuffer<usize>, LockedBuffer<T>), Error=Error>> {
      // Step 1. Sync all input buffers to the required device
      let dev = self.device();
      let ar = shape_a.sync(dev).join(a.sync(dev));
      let br = shape_b.sync(dev).join(b.sync(dev));

      // Step 2. Convert all memory to native memory and execute the
      //   broadcasted dot operation on the cpu pool
      let dev = self.device().clone();
      let pool = self.device().pool().clone();
      Box::new(ar.join(br).and_then(move |((shape_a, a), (shape_b, b))| {
        pool.spawn_fn(move || {
          let (shape_c, c) = {
            let n_shape_a: &[usize] = try!(try!(shape_a.native_memory(&dev)).try_as_slice());
            let n_a: &[T] = try!(try!(a.native_memory(&dev)).try_as_slice());
            let n_shape_b: &[usize] = try!(try!(shape_b.native_memory(&dev)).try_as_slice());
            let n_b: &[T] = try!(try!(b.native_memory(&dev)).try_as_slice());
            //let n_shape_c: &mut [usize] = try!(try!(shape_c.native_memory_mut(&dev)).try_as_mut_slice());
            //let n_c: &mut [T] = try!(try!(c.native_memory_mut(&dev)).try_as_mut_slice());

            let (mut bshape, iter_a, iter_b) = try!(broadcast::try_new_broadcast(n_shape_a, n_a, n_shape_b, n_b, 1));
            bshape.pop();

            let mut c = try!(try!(Buffer::with_capacity_native(&dev, bshape.iter().sum())).try_lock());
            let shape_c = try!(try!(Buffer::from_vec_native(&dev, bshape)).try_lock());

            {
              let n_c: &mut [T] = try!(try!(c.native_memory_mut(&dev)).try_as_mut_slice());

              let r_iter = iter_a.zip(iter_b).map(|(a, b)| T::dot(a, b));

              for(v1, v2) in r_iter.zip(n_c.iter_mut()) {
                *v2 = v1;
              }
            }

            (shape_c, c)
          };

          Ok((shape_c, c))
        })
      }))
    }
}
