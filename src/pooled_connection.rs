use super::prelude::*;
use dbus::{Connection, BusType, Path, ConnPath, BusName};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
    last_event: HashMap<String, Instant>,
}

impl PooledConnection {
    pub(crate) fn new(connection: Connection) -> Self {
        let _ = connection.add_match(
            "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'",
        );
        PooledConnection {
            connection: connection,
            last_event: HashMap::new(),
        }
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
        PooledConnection::new(connection)
    }
}
