use actix_web::Scope;

use crate::server::generator;
use crate::server::nodes;

pub fn route() -> Scope {
    Scope::new("/api")
        .service(generator::route())
        .service(nodes::route())
}
