use std::{ future::{ready, Ready}, rc::Rc};

use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, error::ErrorUnauthorized, web, Error, HttpMessage};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::{models::{Claims, SubscriptionData}, AppState};

#[derive(Clone)]
pub struct VerifyJWT {
    app_data: web::Data<AppState>,
}

impl VerifyJWT {
    pub fn new(app_data: web::Data<AppState>) -> Self {
        Self { app_data }
    }
}

impl<S, B> Transform<S, ServiceRequest> for VerifyJWT
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = VerifyJWTMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(VerifyJWTMiddleware {
            service: Rc::new(service),
            app_data: self.app_data.clone(),
        }))
    }
}

pub struct VerifyJWTMiddleware<S> {
    service: Rc<S>,
    app_data: web::Data<AppState>,
}

impl<S, B> Service<ServiceRequest> for VerifyJWTMiddleware<S>
where 
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static ,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| {
                if s.starts_with("Bearer ") {
                    Some(s[7..].to_string())
                } else {
                    None
                }
            });

        if let Some(token) = auth_header {
            match decode::<Claims>(
                &token, 
                &DecodingKey::from_secret(self.app_data.jwt_access_secret.as_bytes()), 
                &Validation::default()
            ) {
                Ok(data) => {
                    let pool = self.app_data.db.clone(); 
                    let user = data.claims.user.clone();
                    let svc = self.service.clone();

                    Box::pin(async move { 
                        req.extensions_mut().insert(user.clone());
                        let sub = sqlx::query_as!(
                            SubscriptionData,
                            r#"
                            SELECT 
                                plan AS "plan: crate::models::SubscriptionPlan", 
                                status AS "status: crate::models::SubscriptionStatus", 
                                ends_at
                            FROM subscriptions
                            WHERE user_id = $1
                            "#,
                            user.id
                        )
                        .fetch_one(&pool)
                        .await
                        .map_err(actix_web::error::ErrorInternalServerError)?;

                        req.extensions_mut().insert(sub);
                        
                        let fut = svc.call(req);
                        fut.await 
                    })
                }
                Err(_) => {
                    Box::pin(async {
                        Err(ErrorUnauthorized("Invalid or expired token"))
                    })
                }
            }
        } else {
            Box::pin(async {
                Err(ErrorUnauthorized("Missing Bearer token"))
            })
        }
    }
}