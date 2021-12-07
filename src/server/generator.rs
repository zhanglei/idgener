use actix_web::{get, web, Responder, Result, Scope};

use crate::generator::Idgend;
use crate::server::ext::Actix;
use crate::server::AppState;

pub fn route() -> Scope {
    Scope::new("/g").service(snowflake)
}

#[get("/snowflake")]
pub async fn snowflake(data: web::Data<AppState>) -> Result<impl Responder> {
    let snowflake = *data.snowflake.read().actix()?;
    if let None = snowflake {
        return Err(actix_web::error::ErrorNotFound("not ready"));
    }
    snowflake.unwrap().get(true).actix()
}

#[cfg(test)]
mod tests {
    use crate::cluster::Node;
    use actix_web::{test, web, App};

    use crate::config::logger;
    use crate::generator::Snowflake;
    use crate::server::generator::snowflake;
    use crate::server::AppState;

    #[actix_rt::test]
    async fn test_body() {
        logger::init(true);
        let srv = test::start(|| {
            let state = web::Data::new(AppState::default());
            state
                .nodes
                .write()
                .unwrap()
                .join(Node::new(0, "127.0.0.1:1024".parse().unwrap()))
                .set_leader(Some(0))
                .set_current(0);
            let _ = state.snowflake.write().unwrap().insert(Snowflake::new(0));
            App::new().app_data(state).service(snowflake)
        });
        let request = srv.get("/snowflake");
        let response = request.send().await.unwrap();
        assert!(
            response.status().is_success(),
            "response status code is not 200"
        );
    }
}
