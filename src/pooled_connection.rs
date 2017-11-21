use super::prelude::*;
use dbus::{Connection, BusType, Path, ConnPath, BusName};

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
}

impl PooledConnection {
    pub(crate) fn new() -> Result<Self> {
        Ok(Connection::get_private(BusType::Session)?.into())
    }

    pub(crate) fn with_path<'a>(
        &'a self,
        bus_name: BusName<'a>,
        path: Path<'a>,
        timeout_ms: i32,
    ) -> ConnPath<&'a Connection> {
        self.connection.with_path(bus_name, path, timeout_ms)
    }

    pub(crate) fn underlying(&self) -> &Connection {
        &self.connection
    }
}

impl From<Connection> for PooledConnection {
    fn from(connection: Connection) -> Self {
        PooledConnection { connection: connection }
    }
}
