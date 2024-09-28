use actix_session::{Session, SessionExt, SessionGetError};
use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, error::ErrorForbidden};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use tracing::Instrument;

pub struct TypedSession(Session);

impl TypedSession {
    pub fn get(&self, key: &str) -> Result<Option<String>, SessionGetError>{
        self.0.get(key)
    }
}

pub struct SessionMiddlewareFactory;

impl<S> Transform<S, ServiceRequest> for SessionMiddlewareFactory
where 
    S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = RouteSessionMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RouteSessionMiddleware{service}))
    }
}

pub struct RouteSessionMiddleware<S>{
    service: S
}

impl<S> Service<ServiceRequest> for RouteSessionMiddleware<S>
where 
    S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>,
    S::Future: 'static
{
        type Error = actix_web::Error;
        type Response = S::Response;
        type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

        forward_ready!(service);

        #[tracing::instrument(
            "Checking if user is authenticated to access service",
            skip(self, req)
        )]
        fn call(&self, req: ServiceRequest) -> Self::Future {
            let session = TypedSession(req.get_session());
            let user_id_option = session.get("user_id").unwrap();

            let current_span = tracing::Span::current();

            if user_id_option.is_none(){
                return Box::pin(ready(
                    Err(ErrorForbidden("Not logged in"))
                ).instrument(current_span))
            }


            let fut = self.service.call(req);

            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            }
            .instrument(current_span))
        }
}
