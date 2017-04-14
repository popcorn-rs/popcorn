use uuid::Uuid;
use exec::*;
use std::marker::PhantomData;
use futures::Future;

pub struct Placeholder<T: Send + Copy + 'static> {
  uid: Uuid,

  _pd: PhantomData<T>
}

impl<T: Send + Copy + 'static> Placeholder<T> {
  pub fn new() -> Placeholder<T> {
    Placeholder {
      uid: Uuid::new_v4(),
      _pd: PhantomData { }
    }
  }
}

impl<T: Send + Copy + 'static> Executable for Placeholder<T> {
  type Base = T;

  fn uid(&self) -> &Uuid { &self.uid }
  fn exec<'a>(&self, ctx: &'a mut Context) ->
    Result<Box<Future<Item=Buffer<Self::Base>,Error=buffer::Error>>,Error> {
      ctx.get_input(self.uid())
    }
}

#[cfg(test)]
mod test {
  use super::*;
  use popcorn::*;
  use exec;

  #[test]
  fn test_placeholder_success() {
    let backend = native::Backend::default();
    let buf: Result<Buffer<f32>, buffer::Error> = Buffer::new(backend.device(), 2).map(|b| {
      b.sync_from_vec(vec![42.0, 32.1], backend.device()).wait().unwrap()
    });

    let p = Placeholder::<f32>::new();

    let mut ctx = exec::Context::new();
    ctx.set_input(p.uid(), buf);

    let dev = backend.device().clone();
    let rv = p.exec(&mut ctx).unwrap().and_then(move |b| {
      b.sync_to_vec(&dev)
    }).map(|(_b, v)| v).wait().unwrap();

    assert_eq!(rv, vec![42.0, 32.1]);
  }
}
