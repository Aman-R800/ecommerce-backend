use actix_session::SessionExt;
use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, error::ErrorForbidden};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use tracing::Instrument;

use crate::session_state::TypedSession;

pub struct AdminMiddlewareFactory;

impl<S> Transform<S, ServiceRequest> for AdminMiddlewareFactory
where 
    S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>,
    S::Future: 'static
{
    type Response = ServiceResponse;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = AdminMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AdminMiddleware{service}))
    }
}


pub struct AdminMiddleware<S>{
    service: S
}

impl<S> Service<ServiceRequest> for AdminMiddleware<S>
where 
    S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error>,
    S::Future : 'static
{
    type Response = S::Response;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    #[tracing::instrument(
        "Checking if user is admin",
        skip(self, req)
    )]
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let session = TypedSession(req.get_session());
        let is_admin_option = session.get("is_admin").unwrap();

        let current_span = tracing::Span::current();

        if is_admin_option.is_none(){
            return Box::pin(
                ready(Err(ErrorForbidden("Not authorized")))
                    .instrument(current_span)
            )
        }

        let fut = self.service.call(req);

        Box::pin(
            async move {
                let res = fut.await?;
                Ok(res)
            }
            .instrument(current_span)
        )
    }
}
