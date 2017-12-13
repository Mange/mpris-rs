use dbus::{Connection, Path, ConnPath, BusName, Message};
use player::MPRIS2_PATH;
use progress::DurationExtensions;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
    last_tick: Instant,
    last_event: RefCell<HashMap<String, Instant>>,
}

const GET_NAME_OWNER_TIMEOUT: i32 = 100; // ms

impl PooledConnection {
    pub(crate) fn new(connection: Connection) -> Self {
        let _ = connection.add_match(
            "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'",
        );
        let _ = connection.add_match(
            "interface='org.mpris.MediaPlayer2.Player',member='Seeked',path='/org/mpris/MediaPlayer2'",
        );
        PooledConnection {
            connection: connection,
            last_tick: Instant::now(),
            last_event: RefCell::new(HashMap::new()),
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

    pub(crate) fn determine_unique_name<S: Into<String>>(&self, bus_name: S) -> Option<String> {
        let get_name_owner = Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "GetNameOwner",
        ).unwrap()
            .append1(bus_name.into());

        self.connection
            .send_with_reply_and_block(get_name_owner, GET_NAME_OWNER_TIMEOUT)
            .ok()
            .and_then(|reply| reply.get1())
    }

    pub(crate) fn last_event_for_unique_name(&self, unique_name: &str) -> Option<Instant> {
        self.last_event.borrow().get(unique_name).cloned()
    }

    pub(crate) fn process_events_blocking(&self, duration: Duration) {
        // Try to read messages util time is up. Keep going with smaller and smaller windows until
        // our time is up.
        let start = Instant::now();

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
                Some(message) => {
                    if PooledConnection::is_watched_message(&message) {
                        self.process_message(message);
                    }
                }
                None => {
                    // Time is up. No more messages.
                    break;
                }
            }
        }
    }

    fn is_watched_message(message: &Message) -> bool {
        use std::ops::Deref;

        if let Some(message_path) = message.path().as_ref() {
            if message_path.deref() != MPRIS2_PATH {
                return false;
            }
        }

        if let Some(message_member) = message.member().as_ref() {
            if message_member.deref() == "Seeked" || message_member.deref() == "PropertiesChanged" {
                return true;
            }
        }

        return false;
    }

    fn process_message(&self, message: Message) {
        message.sender().map(|unique_name| {
            self.mark_bus_as_updated((*unique_name).to_owned())
        });
    }

    fn mark_bus_as_updated<S: Into<String>>(&self, bus_name: S) {
        self.last_event.borrow_mut().insert(
            bus_name.into(),
            Instant::now(),
        );
    }
}

impl From<Connection> for PooledConnection {
    fn from(connection: Connection) -> Self {
        PooledConnection::new(connection)
    }
}
