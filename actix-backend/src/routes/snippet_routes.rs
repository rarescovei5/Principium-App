use actix_web::web;

use crate::{handlers::snippet_handler, middleware::jwt_middleware::VerifyJWT};

pub fn config(config: &mut web::ServiceConfig, jwt_middleware: VerifyJWT) {
    config.service(
        web::scope("/v1/users")
        .service(snippet_handler::create_snippet)
        .service(snippet_handler::get_user_snippet)
        .service(snippet_handler::get_user_snippets)
        .service(snippet_handler::update_snippet)
        .service(snippet_handler::delete_snippet)
        .wrap(jwt_middleware.clone())
    ).service(
        web::scope("/v1/snippets")
        .service(snippet_handler::star_snippet)
        .service(snippet_handler::unstar_snippet)
        .wrap(jwt_middleware)
    ).service(
        web::scope("/v1/public/snippets")
        .service(snippet_handler::get_page_snippets)
        .service(snippet_handler::get_snippets_by_ids)
    );
}