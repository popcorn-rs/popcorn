//use std::sync::Arc;
//use std::ops::Deref;
//use std::marker::PhantomData;
//use device::Device;
//use event::Event;

//pub struct Buffer<T> {
  //inner: Arc<Inner<T>>
//}

//pub struct Inner<T> {
  //device: Box<Device>,
  //latest_event: Box<Event>,
  //_pd: PhantomData<T>
//}

//impl<T> Deref for Buffer<T> {
  //type Target = Inner<T>;

  //fn deref(&self) -> &Self::Target {
    //&self.inner
  //}
//}

//impl<T: Copy> Buffer<T> {
  //pub fn new<D: Device>(device: D, size: usize) -> Buffer<T> {
    //let dev = Box::new(device) as Box<Device>;
    //// Allocate device memory/event for memory
    //// Return created buffer
  //}
//}
