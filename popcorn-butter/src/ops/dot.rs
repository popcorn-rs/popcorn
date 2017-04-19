use uuid::Uuid;
use exec::*;
use std::marker::PhantomData;
use futures::Future;

pub struct Dot<T: Send + Copy + 'static> {
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
    Result<Box<Future<Item=LockedBuffer<Self::Base>,Error=buffer::Error>>,Error> {
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
    let buf = Buffer::<f32>::new(backend.device(), 2).unwrap().lock().and_then(|b| {
      b.sync_from_vec(vec![42.0, 32.1])
    });

    let p = Placeholder::<f32>::new();

    let mut ctx = exec::Context::new();
    ctx.set_input(p.uid(), buf);

    let rv = p.exec(&mut ctx).unwrap().and_then(move |b| {
      b.sync_to_vec()
    }).map(|v| v).wait().unwrap();

    assert_eq!(rv, vec![42.0, 32.1]);
  }
}

