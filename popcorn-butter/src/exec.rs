use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::ops::Deref;
use futures::{Future, IntoFuture};
use futures::future::Shared;
use uuid::Uuid;

pub use std::any::Any;
pub use popcorn::buffer::{self, Buffer};

/// (S)hared (B)oxed (B)uffer (F)uture
pub type SBBF<Base> = Shared<Box<Future<Item=Buffer<Base>,Error=buffer::Error>>>;

#[derive(Debug, Clone)]
pub enum Error {
  PlaceholderError,
  NoSuchElement,
  Buffer(buffer::Error),
  DowncastError
}

pub struct Context {
  cache: HashMap<Uuid, Vec<Box<Any>>>,
}

pub trait Executable {
  fn uid(&self) -> &Uuid;
  fn exec<'a>(&self, ctx: &'a mut Context) -> Result<Vec<Box<Any>>, Error>;
}

pub struct Socket<Base: Send + Sized + Copy + 'static> {
  executable: Arc<Executable>,
  index: usize,

  _pd: PhantomData<Base>
}

impl<Base: Send + Sized + Copy + 'static> Socket<Base> {
  pub fn new(e: Arc<Executable>, index: usize) -> Socket<Base> {
    Socket {
      executable: e,
      index: index,

      _pd: PhantomData { }
    }
  }

  pub fn exec(&self, ctx: &mut Context) -> Result<SBBF<Base>, Error> {
    ctx.try_caching(self.executable.deref(), self.index)
  }
}

impl Context {
  pub fn new() -> Context {
    Context {
      cache: HashMap::new()
    }
  }

  pub fn cache_executable(&mut self, e: &Executable) -> Result<(), Error> {
    if !self.cache.contains_key(e.uid()) {
      let items = try!(e.exec(self));
      self.cache.insert(e.uid().clone(), items);
    }

    Ok(())
  }

  pub fn try_caching<Base: Send + Sized + Copy + 'static>(&mut self,
                                                          e: &Executable,
                                                          index: usize) -> Result<SBBF<Base>, Error> {
    try!(self.cache_executable(e));

    self.cache.get(e.uid()).and_then(|b| b.get(index)).and_then(|b| {
      b.downcast_ref::<SBBF<Base>>().map(|b| Ok(b.clone()))
    }).unwrap_or_else(|| Err(Error::NoSuchElement))
  }

  pub fn set_input<Base: Send + Copy + 'static,
  B: IntoFuture<Item=Buffer<Base>,Error=buffer::Error> + 'static>(&mut self,
                                                                  uid: Uuid,
                                                                  buf: B) -> &mut Self {
    let f: Box<Future<Item=Buffer<Base>,Error=buffer::Error>> = Box::new(buf.into_future());
    let bf = Box::new(f.shared()) as Box<Any>;
    self.cache.insert(uid, vec![bf]);
    self
  }
}
