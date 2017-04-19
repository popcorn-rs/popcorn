use uuid::Uuid;
use exec::*;
use std::marker::PhantomData;

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
  fn uid(&self) -> &Uuid { &self.uid }
  fn exec<'a>(&self, _ctx: &'a mut Context) ->
    Result<Vec<Box<Any>>, Error> { Err(Error::PlaceholderError) }
}

#[cfg(test)]
mod test {
  use super::*;
  use popcorn::*;
  use futures::Future;
  use std::sync::Arc;
  use exec;

  #[test]
  fn test_placeholder_success() {
    let backend = native::Backend::default();
    let buf = Buffer::<f32>::new(backend.device(), 2).unwrap();
    let buff = buf.lock().and_then(|b| {
      b.sync_from_vec(vec![42.0, 32.1])
    }).map(|_| buf);

    let p = Arc::new(Placeholder::<f32>::new());

    let mut ctx = exec::Context::new();
    ctx.set_input(p.uid().clone(), buff);
    let socket = Socket::<f32>::new(p.clone() as Arc<Executable>, 0);

    let rv = socket.exec(&mut ctx).unwrap().
      map_err(|se| se.clone()).
      and_then(|b| {
        b.lock().and_then(|b| b.sync_to_vec())
      }).map(|v| v).wait().unwrap();

    assert_eq!(rv, vec![42.0, 32.1]);
  }
}
