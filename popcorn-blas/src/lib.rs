extern crate futures;
extern crate popcorn;
extern crate blas_sys;

pub mod operation;
pub mod frameworks;

pub use operation::*;
pub use frameworks::native::*;

#[cfg(test)]
mod native_test {
  use popcorn;
  use popcorn::*;
  use super::*;
  use futures::Future;

  #[test]
  fn dot_test() {
    let backend = popcorn::frameworks::native::Backend::default();

    let shape_vec = vec![1, 4];

    let shape_a: LockedBuffer<usize> = Buffer::new(backend.device(), 2).unwrap().lock().and_then(|b| b.sync_from_vec(shape_vec.clone())).wait().unwrap();
    let a: LockedBuffer<f32> = Buffer::new(backend.device(), 4).unwrap().lock().and_then(|b| b.sync_from_vec(vec![1.0, 2.0, 3.0, 4.0])).wait().unwrap();
    let shape_b: LockedBuffer<usize> = Buffer::new(backend.device(), 2).unwrap().lock().and_then(|b| b.sync_from_vec(shape_vec.clone())).wait().unwrap();
    let b: LockedBuffer<f32> = Buffer::new(backend.device(), 4).unwrap().lock().and_then(|b| b.sync_from_vec(vec![2.0, 2.0, 2.0, 2.0])).wait().unwrap();
    let shape_c: LockedBuffer<usize> = Buffer::new(backend.device(), 0).unwrap().lock().wait().unwrap();
    let c: LockedBuffer<f32> = Buffer::new(backend.device(), 1).unwrap().lock().wait().unwrap();

    let (_shape_aa, _aa, _shape_bb, _bb, shape_c, c) = backend.bcast_dot(shape_a, a,
                                                                         shape_b, b,
                                                                         shape_c, c).wait().unwrap();

    let shape_c_vec = shape_c.native_memory(backend.device()).unwrap().try_as_slice::<usize>().unwrap();
    let c_vec = c.native_memory(backend.device()).unwrap().try_as_slice::<f32>().unwrap();
    println!("Shape: {:?}", &shape_c_vec);
    println!("Contents: {:?}", &c_vec);
  }
}
