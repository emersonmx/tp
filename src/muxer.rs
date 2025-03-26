use crate::config::Session;
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum Error {}

pub fn apply(session: Session) -> Result<(), Error> {
    println!("{:?}", session);
    Ok(())
}
