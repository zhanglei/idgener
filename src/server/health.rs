use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

use crate::server::ext::Actix;
use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum HealthState {
    Prepare,
    Running,
}

///监控数据
#[derive(Debug, Serialize, Deserialize)]
pub struct Health {
    pub state: HealthState,
    pub leader: Option<u16>,
    pub id: u16,
}

pub async fn health(data: web::Data<AppState>) -> Result<HttpResponse> {
    let nodes = &*data.nodes.read().actix()?;
    return match nodes.get_current() {
        None => Ok(HttpResponse::InternalServerError().json(Health {
            state: HealthState::Prepare,
            leader: None,
            id: 0,
        })),
        Some(current) => {
            return Ok(HttpResponse::Ok().json(Health {
                state: HealthState::Running,
                leader: nodes.get_leader().and_then(|n| Some(n.id)),
                id: current.id,
            }));
        }
    };
}
