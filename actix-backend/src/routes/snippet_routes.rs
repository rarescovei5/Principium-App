use actix_web::web;

use crate::{handlers, middleware::jwt_middleware::VerifyJWT};

pub fn config(config: &mut web::ServiceConfig, jwt_middleware: VerifyJWT) {
    config.service(
        web::scope("/v1/users")
        .wrap(jwt_middleware.clone())
        .service(handlers::snippet_handler::create_snippet)
        .service(handlers::snippet_handler::get_user_snippet)
        .service(handlers::snippet_handler::get_user_snippets)
        .service(handlers::snippet_handler::update_snippet)
        .service(handlers::snippet_handler::delete_snippet)
    ).service(
        web::scope("/v1/snippets")
        .service(handlers::snippet_handler::get_page_snippets)
        .wrap(jwt_middleware)
        .service(handlers::snippet_handler::star_snippet)
        .service(handlers::snippet_handler::unstar_snippet)
    );
}