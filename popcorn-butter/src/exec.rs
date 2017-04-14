use std::collections::HashMap;
use std::any::Any;
use futures::Future;
use popcorn::buffer::{self, Buffer};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum Error {
  InvalidInput
}

pub struct Context {
  inputs: HashMap<String, Box<Any>>,
}

pub trait Executable {
  type Base: Send + Copy + 'static;

  fn uid(&self) -> &Uuid;
  fn exec<'a>(&self, ctx: &'a mut Context) -> Result<Box<Future<Item=Buffer<Self::Base>,Error=buffer::Error>>,Error>;
}

impl Context {
  pub fn get_input<Base: Send + Copy + 'static>(&mut self,
                                                name: &str) ->
    Result<Box<Future<Item=Buffer<Base>,Error=buffer::Error>>,Error> {
      self.inputs.remove(name).map(|input| {
        input.downcast::<Box<Future<Item=Buffer<Base>,Error=buffer::Error>>>().
          map(|x| *x).
          map_err(|_| Error::InvalidInput)
      }).unwrap_or_else(|| Err(Error::InvalidInput))
    }
}
