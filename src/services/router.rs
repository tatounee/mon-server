use std::mem;
use std::str::FromStr;
use std::task::Poll;

use color_eyre::eyre::Report;
use http::uri::PathAndQuery;
use http::{Request, Response, StatusCode, Uri};
use tower::Layer;
use tower::layer::util::{Identity, Stack};
use tower::{Service, util::BoxCloneService};

use crate::body::Body;
use crate::utils::basic_response;

type RouteService<Req, Res, Err> = BoxCloneService<Req, Res, Err>;

pub struct Router<L, Req, Res, Err> {
    layer: L,
    routes: Vec<Route<Req, Res, Err>>,
}

struct Route<Req, Res, Err> {
    path: String,
    service: RouteService<Req, Res, Err>,
}
impl<Req, Res, Err> Router<Identity, Req, Res, Err> {
    pub fn new() -> Self {
        Self {
            layer: Identity::new(),
            routes: Vec::new(),
        }
    }
}

impl<L, Req, Res, Err> Router<L, Req, Res, Err> {
    pub fn route<U, S, F>(mut self, uri: U, service: S) -> Self
    where
        U: Into<String>,
        S: Service<Req, Response = Res, Error = Err, Future = F> + Send + Clone + 'static,
        F: Send + 'static,
    {
        let service = BoxCloneService::new(service);

        let route = Route {
            path: uri.into(),
            service,
        };

        self.routes.push(route);
        self
    }

    pub fn layer<T>(self, layer: T) -> Router<Stack<T, L>, Req, Res, Err> {
        Router {
            layer: Stack::new(layer, self.layer),
            routes: self.routes,
        }
    }
}

impl<L, S, B> Service<Request<B>> for Router<L, Request<B>, Response<Body>, Report>
where
    L: Layer<RouteService<Request<B>, Response<Body>, Report>, Service = S> + Clone,
    S: Service<Request<B>, Response = Response<Body>, Error = Report>,
{
    type Response = Response<Body>;

    type Error = Report;

    type Future = impl Future<Output = Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self
            .routes
            .iter_mut()
            .all(|route| matches!(route.service.poll_ready(cx), Poll::Ready(_)))
        {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let route = self.routes.iter_mut().find(|route| {
            if req.uri().path().starts_with(&route.path) {
                let mut uri = mem::take(req.uri_mut()).into_parts();

                let path_and_query = uri.path_and_query.unwrap();
                let striped_path = path_and_query.path().strip_prefix(&route.path).unwrap();

                let sep = if path_and_query.query().is_some() {
                    "?"
                } else {
                    ""
                };

                let query = path_and_query.query().unwrap_or_default();

                uri.path_and_query =
                    PathAndQuery::from_str(&format!("{striped_path}{sep}{query}")).ok();

                *req.uri_mut() = Uri::from_parts(uri).unwrap();

                true
            } else {
                false
            }
        });

        let layer = self.layer.clone();

        let service = route.map(|route| {
            let cloned_service = route.service.clone();
            mem::replace(&mut route.service, cloned_service)
        });

        async move {
            match service {
                Some(service) => {
                    let mut service = layer.layer(service);
                    Service::call(&mut service, req).await
                }
                None => Ok(basic_response(StatusCode::NOT_FOUND)),
            }
        }
    }
}

impl<L, Req, Res, Err> Clone for Router<L, Req, Res, Err>
where
    L: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
            routes: self.routes.clone(),
        }
    }
}

impl<Req, Res, Err> Clone for Route<Req, Res, Err> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            service: self.service.clone(),
        }
    }
}
