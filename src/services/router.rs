use std::mem;
use std::str::FromStr;
use std::task::Poll;

use bytes::Bytes;
use color_eyre::eyre::Report;
use http::uri::PathAndQuery;
use http::{Request, Response, StatusCode, Uri};
use tower::{Service, util::BoxCloneService};

use crate::utils::basic_response;

type RouteService<Req, Res, Err> = BoxCloneService<Req, Res, Err>;

pub struct Router<Req, Res, Err> {
    routes: Vec<Route<Req, Res, Err>>,
}

struct Route<Req, Res, Err> {
    path: String,
    service: RouteService<Req, Res, Err>,
}

impl<Req, Res, Err> Router<Req, Res, Err> {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

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
}

impl<B> Service<Request<B>> for Router<Request<B>, Response<Bytes>, Report> {
    type Response = Response<Bytes>;

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

        let service = route.map(|route| {
            let cloned = route.service.clone();
            mem::replace(&mut route.service, cloned)
        });

        // let mut serice = match route {
        //     Some(route) => {
        //     }
        //     None => {
        //         todo!()
        //     }
        // };

        async move {
            match service {
                Some(mut service) => service.call(req).await,
                None => Ok(basic_response(StatusCode::NOT_FOUND)),
            }
        }
    }
}

impl<Req, Res, Err> Clone for Router<Req, Res, Err> {
    fn clone(&self) -> Self {
        Self {
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
