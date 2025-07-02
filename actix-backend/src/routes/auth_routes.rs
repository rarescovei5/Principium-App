use actix_web::web;

use crate::handlers::auth_handler;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/v1/auth")
        .service(auth_handler::register)
        .service(auth_handler::login)
        .service(auth_handler::logout)
        .service(auth_handler::refresh)
    );
}