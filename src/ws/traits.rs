use serde::Serialize;

pub trait Subscription: Serialize + Send + Sync {
    fn channel(&self) -> &'static str;
}
