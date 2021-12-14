use std::hash::{Hash, Hasher};

pub trait MaybeFrom<T>: Sized {
    fn maybe_from(from: T) -> Option<Self>;
}

pub(crate) struct StreamSubscription<T> {
    s: T,
}

impl<T> StreamSubscription<T> {
    pub fn new(stream: T) -> Self {
        Self { s: stream }
    }
}

impl<H, I, T> iced_native::subscription::Recipe<H, I> for StreamSubscription<T>
where
    H: Hasher,
    T: 'static + futures::Stream + Send + Sync,
{
    type Output = <T as futures::Stream>::Item;

    fn hash(&self, state: &mut H) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: iced_futures::BoxStream<I>,
    ) -> iced_futures::BoxStream<Self::Output> {
        Box::pin(self.s)
    }
}

trait InsertIntoInner {
    type Value;
    fn insert_into_inner(&mut self, value: Self::Value);
}

impl<Key, Value> InsertIntoInner for std::collections::HashMap<Key, Vec<Value>>
where
    Key: Hash + Eq,
{
    type Value = (Key, Value);

    fn insert_into_inner(&mut self, value: Self::Value) {
        match self.get_mut(&value.0) {
            Some(vec) => vec.push(value.1),
            None => {
                self.insert(value.0, vec![value.1]);
            }
        }
    }
}
