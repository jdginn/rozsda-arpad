/// Stub traits matching the generated code's imports.
/// These are simplified versions used in tests; the main arpad-rust crate
/// provides the full versions with additional bounds (e.g. Send).
pub mod traits {
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
}

/// Stub OSC module providing the traits the generated context modules import.
pub mod osc {
    pub mod route_context {
        pub trait ContextTrait: std::fmt::Debug + Eq + Clone + std::hash::Hash {}

        pub trait ContextKindTrait: std::fmt::Debug + Eq + Clone + std::hash::Hash {
            type Context: ContextTrait + 'static;

            fn parse(osc_address: &str) -> Option<Self::Context>
            where
                Self: Sized;

            fn context_name() -> &'static str;
        }
    }
}
