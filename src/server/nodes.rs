
use std::net::SocketAddr;


use crate::cluster::{Node};
use actix_web::{get, post, web, HttpResponse, Result, Scope};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::server::{ext::Actix, AppState};

pub fn route() -> Scope {
    Scope::new("/nodes").service(all).service(join)
}

#[get("")]
pub async fn all(data: web::Data<AppState>) -> Result<HttpResponse> {
    let nodes = &*data.nodes.read().actix()?;
    Ok(HttpResponse::Ok().json(nodes))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinInfo {
    pub address: SocketAddr,
    pub current_id: Option<u16>,
    pub nodes: Option<Vec<Node>>,
}

impl Into<actix_http::body::Body> for JoinInfo {
    fn into(self) -> actix_http::body::Body {
        actix_http::body::Body::from(serde_json::to_string(&self).expect("serialize JoinInfo"))
    }
}

impl Into<http_client::Body> for JoinInfo {
    fn into(self) -> http_client::Body {
        http_client::Body::from_json(&self).unwrap()
    }
}

#[post("")]
pub async fn join(body: web::Json<JoinInfo>, state: web::Data<AppState>) -> Result<HttpResponse> {
    let mut nodes = state.nodes.write().unwrap();
    if nodes.self_is_none() {
        return Err(actix_web::error::ErrorInternalServerError("not ready"));
    }

    if let Some(node) = nodes.get_node_by_address(&body.address) {
        log::debug!("node keep-alive: [{}]:{}", &node.id, &node.address);
        node.last_alive_timestamp = Local::now().timestamp_millis();
        return Ok(HttpResponse::Ok().body(node.id.to_string()));
    }

    log::info!("not found node: {}, joining", &body.address);

    let new_id = body.current_id.or(Some(nodes.new_node_id())).unwrap();

    nodes.join(Node::new(new_id, body.address));

    Ok(HttpResponse::Ok().body(JoinInfo {
        address: nodes.get_leader().unwrap().address,
        current_id: Some(new_id),
        nodes: Some(nodes.nodes.clone()),
    }))
}

#[cfg(test)]
mod test {
    use crate::cluster::{Node};
    use crate::config::logger;
    use crate::generator::Snowflake;
    use crate::server::nodes::{route, JoinInfo};
    use crate::server::AppState;
    use actix_web::test::TestServer;
    use actix_web::web::Buf;
    use actix_web::{test, web, App};
    use std::net::SocketAddr;

    #[test]
    fn nodes_serialize() {
        let out = JoinInfo {
            address: "127.0.0.1:1024".parse().unwrap(),
            current_id: Some(1),
            nodes: Some(vec![Node::new(1, "127.0.0.1:1025".parse().unwrap())]),
        };
        let json = serde_json::to_string_pretty(&out).unwrap();
        println!("json: {}", json);

        let info = serde_json::from_str::<JoinInfo>(json.as_str());
        print!("info: {:?}", info);
    }

    async fn send(srv: &TestServer) {
        let request = srv.post("/nodes");
        let current_id = 3;
        let mut response = request
            .content_type("application/json")
            .send_body(JoinInfo {
                address: "127.0.1.1:1024".parse::<SocketAddr>().unwrap(),
                current_id: Some(current_id),
                nodes: None,
            })
            .await
            .unwrap();
        assert!(
            response.status().is_success(),
            "response status code is not 200"
        );
        let id = response.body().await;
        assert!(id.is_ok(), "send request");
        let id = id.unwrap();
        let info = serde_json::from_slice::<JoinInfo>(id.bytes());
        println!("out: {:?}", info);
        assert!(info.is_ok());
        let info = info.unwrap();
        println!("body: {:?}", info);
        assert_eq!(Some(current_id), info.current_id);
    }

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
            App::new().app_data(state).service(route())
        });
        send(&srv).await;
    }
}
