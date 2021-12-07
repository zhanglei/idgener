use std::sync::RwLock;

pub use server::embedded;

use crate::cluster::Nodes;
use crate::generator::Snowflake;

mod generator;
mod health;
mod nodes;
mod routers;
mod server;

pub struct AppState {
    pub snowflake: RwLock<Option<Snowflake>>,
    pub nodes: RwLock<Nodes>,
}

impl AppState {
    pub fn default() -> Self {
        AppState {
            snowflake: RwLock::new(None),
            nodes: RwLock::new(Nodes::default()),
        }
    }
}

mod ext {
    use std::sync::LockResult;

    pub trait Actix<T> {
        fn actix(self) -> actix_web::Result<T>;
    }

    impl<T> Actix<T> for anyhow::Result<T> {
        fn actix(self) -> actix_web::Result<T> {
            self.map_err(|error| actix_http::error::ErrorExpectationFailed(error.to_string()))
        }
    }

    impl<T> Actix<T> for LockResult<T> {
        fn actix(self) -> actix_web::Result<T> {
            self.map_err(|error| actix_http::error::ErrorExpectationFailed(error.to_string()))
        }
    }

    impl<T> Actix<T> for Option<T> {
        fn actix(self) -> actix_web::Result<T> {
            match self {
                Some(r) => Ok(r),
                None => Err(actix_http::error::ErrorNotFound("not ready")),
            }
        }
    }
}
