use super::prelude::*;
use dbus::{Connection, BusType, Path, ConnPath, BusName};
use progress::DurationExtensions;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
    last_tick: Instant,
    last_event: HashMap<String, Instant>,
}

impl PooledConnection {
    pub(crate) fn new(connection: Connection) -> Self {
        let _ = connection.add_match(
            "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'",
        );
        PooledConnection {
            connection: connection,
            last_tick: Instant::now(),
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

    pub(crate) fn process_events_blocking(&self, duration: Duration) -> bool {
        // Try to read messages util time is up. Keep going with smaller and smaller windows until
        // our time is up.
        let start = Instant::now();
        let mut should_refresh = false;

        while start.elapsed() < duration {
            let ms_left = duration
                .checked_sub(start.elapsed())
                .map(|d| d.as_millis())
                .unwrap_or(0);
            // Don't bother if we have very little time left
            if ms_left < 2 {
                break;
            }
            match self.connection.incoming(ms_left as u32).next() {
                Some(n) => {
                    // If it's a matching message, we should refresh.
                    // TODO: Don't refresh on all messages.
                    should_refresh = true;
                }
                None => {
                    // Time is up. No more messages.
                    break;
                }
            }
        }

        should_refresh
    }
}

impl From<Connection> for PooledConnection {
    fn from(connection: Connection) -> Self {
        PooledConnection::new(connection)
    }
}
