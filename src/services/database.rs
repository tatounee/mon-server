use std::any::TypeId;
use std::collections::HashMap;
use std::mem;
use std::sync::LazyLock;

use http::Request;
use tokio::sync::RwLock;
use tower::{Layer, Service};

use crate::typed_map::{TypedMap, Value};

static STORAGE: LazyLock<RwLock<TypedMap>> = LazyLock::new(|| RwLock::new(TypedMap::new()));

#[derive(Clone)]
pub struct Database<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for Database<S>
where
    S: Service<Request<B>> + Clone,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        req.extensions_mut().insert(DbHandler);

        let cloned_service = self.inner.clone();
        let mut service = mem::replace(&mut self.inner, cloned_service);

        async move { service.call(req).await }
    }
}

pub struct DatabaseLayer;

impl<S> Layer<S> for DatabaseLayer {
    type Service = Database<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Database { inner }
    }
}

#[derive(Clone)]
pub struct DbHandler;

impl DbHandler {
    pub async fn insert<C>(&self, key: Value, value: Value) -> Option<Value>
    where
        C: 'static,
    {
        let mut db = (*STORAGE).write().await;
        db.insert::<C>(key, value).await
    }

    pub async fn get<C>(&self, key: Value, callback: impl FnOnce(Option<&Value>))
    where
        C: 'static,
    {
        let db = (*STORAGE).read().await;
        db.get::<C>(key, callback).await
    }
}
