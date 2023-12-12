pub trait State: Sized {
    type Signal;

    fn next(self, signal: &Self::Signal) -> Self;
}
