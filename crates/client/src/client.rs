use std::borrow::Cow;

pub trait Client: Sized {
    fn connect(&mut self, addr: Cow<'_, str>) -> Result<(), Box<dyn std::error::Error>>;
}
