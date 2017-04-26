use uuid::Uuid;
use exec::*;
use futures::Future;

#[cfg(feature = "native")]
pub mod native {
  use super::*;
  use popcorn::Backend;
  use popcorn::native;
  use popcorn_blas::frameworks::native::core_ops;
  use popcorn_blas::*;
  use std::cmp;

  pub struct Dot<T> {
    uid: Uuid,
    shape_a: Buffer<usize>,
    a: Socket<T>,
    shape_b: Buffer<usize>,
    b: Socket<T>,
    backend: native::Backend
  }

  impl<T: core_ops::Dot> Dot<T> {
    pub fn new(shape_a: Vec<usize>,
               a: Socket<T>,
               shape_b: Vec<usize>,
               b: Socket<T>,
               backend: native::Backend) -> Result<Dot<T>, buffer::Error> {
      let b_shape_a = try!(Buffer::from_vec_native(backend.device(), shape_a));
      let b_shape_b = try!(Buffer::from_vec_native(backend.device(), shape_b));

      Ok(Dot {
        uid: Uuid::new_v4(),
        shape_a: b_shape_a,
        a: a,
        shape_b: b_shape_b,
        b: b,
        backend: backend
      })
    }
  }

  impl<T: 'static> Executable for Dot<T> {
    fn uid(&self) -> &Uuid { &self.uid }

    fn exec<'a>(&self, ctx: &'a mut Context) ->
      Result<Vec<Box<Any>>, Error> {
        let sar = self.shape_a.lock();
        let sbr = self.shape_b.lock();
        let ar = try!(self.a.exec(ctx));
        let br = try!(self.b.exec(ctx));
        let backend = self.backend().clone();

        ar.join(br).join(sar.join(sbr)).and_then(move |((a, b), (sa, sb))| {
          let (sc, c) = backend.bcast_dot(sa, a, sb, b);
        });

        Err(Error::PlaceholderError)
      }
  }
}

