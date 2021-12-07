mod snowflake;

use actix_web::Responder;
pub use snowflake::Snowflake;

pub trait Idgend<T>
where
    T: Responder,
{
    /// generator new id
    fn get(&mut self, jump: bool) -> anyhow::Result<T>;
}
