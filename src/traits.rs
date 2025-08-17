pub trait Bind<Args> {
    fn bind<F>(&mut self, callback: F)
    where
        F: FnMut(Args) + 'static;
}

pub trait Set<Args> {
    type Error;
    fn set(&mut self, args: Args) -> Result<(), Self::Error>;
}

pub trait Query {
    type Error;
    fn query(&self) -> Result<(), Self::Error>;
}
