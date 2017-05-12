use super::work;
use event;

pub fn event_result_to_work(result: event::Result) -> work::Result {
  result.map_err(|e| {
    match e {
      event::Error::Custom(c) => work::Error::Custom(c)
    }
  })
}

pub fn work_result_to_event(result: work::Result) -> event::Result {
  result.map_err(|e| {
    match e {
      work::Error::Custom(c) => event::Error::Custom(c)
    }
  })
}
