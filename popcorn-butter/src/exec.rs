use std::collections::HashMap;
use std::any::Any;
use futures::{Future, IntoFuture};
use uuid::Uuid;

pub use popcorn::buffer::{self, LockedBuffer};

#[derive(Debug, Clone, Copy)]
pub enum Error {
  InvalidBufferType,
  UnknownInput
}

pub struct Context {
  inputs: HashMap<Uuid, Box<Any>>,
}

pub trait Executable {
  type Base: Send + Copy + 'static;

  fn uid(&self) -> &Uuid;
  fn exec<'a>(&self, ctx: &'a mut Context) ->
    Result<Box<Future<Item=LockedBuffer<Self::Base>,Error=buffer::Error>>,Error>;
}

impl Context {
  pub fn new() -> Context {
    Context {
      inputs: HashMap::new()
    }
  }

  pub fn set_input<Base: Send + Copy + 'static,
  B: IntoFuture<Item=LockedBuffer<Base>,Error=buffer::Error> + 'static>(&mut self,
                                                                  uid: &Uuid,
                                                                  buf: B) -> &mut Self {
    let f: Box<Future<Item=LockedBuffer<Base>,Error=buffer::Error>> = Box::new(buf.into_future());
    let bf = Box::new(f) as Box<Any>;
    self.inputs.insert(uid.clone(), bf);
    self
  }

  pub fn get_input<Base: Send + Copy + 'static>(&mut self,
                                                uid: &Uuid) ->
    Result<Box<Future<Item=LockedBuffer<Base>,Error=buffer::Error>>,Error> {
      self.inputs.remove(uid).map(|input| {
        input.downcast::<Box<Future<Item=LockedBuffer<Base>,Error=buffer::Error>>>().
          map(|x| *x).
          map_err(|_| Error::InvalidBufferType)
      }).unwrap_or_else(|| Err(Error::UnknownInput))
    }
}
