use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use dbus::{BusName, ConnPath, Connection, Message, Path};

use extensions::DurationExtensions;
use player::MPRIS2_PATH;

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
    last_tick: Instant,
    last_event: RefCell<HashMap<String, Instant>>,
}

const GET_NAME_OWNER_TIMEOUT: i32 = 100; // ms
const NAME_HAS_OWNER_TIMEOUT: i32 = 100; // ms

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

    pub(crate) fn name_has_owner<S: Into<String>>(&self, bus_name: S) -> Option<bool> {
        let name_has_owner = Message::new_method_call(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "NameHasOwner",
        ).unwrap()
            .append1(bus_name.into());

        self.connection
            .send_with_reply_and_block(name_has_owner, NAME_HAS_OWNER_TIMEOUT)
            .ok()
            .and_then(|reply| reply.get1())
    }

    /// Returns `true` is an event has been recorded for the given bus name, after the given
    /// instant.
    ///
    /// If no event have been seen at all for the given bus name, or the last event was on or
    /// before the provided instant then `false` will be returned.
    pub(crate) fn is_bus_updated_after(&self, bus_name: &str, after: &Instant) -> bool {
        self.last_event
            .borrow()
            .get(bus_name)
            .map(|updated_at| updated_at > after)
            .unwrap_or(false)
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
            if let Some(message) = self.connection
                .incoming(ms_left as u32)
                .filter(PooledConnection::is_watched_message)
                .next()
            {
                self.process_message(&message);
            }
        }
    }

    /// Block until a MPRIS2 event for the given unique bus name is detected.
    ///
    /// Events for other buses will also be recorded, but the method will not return until a
    /// matching one has been found.
    pub(crate) fn process_events_blocking_until_dirty(&self, unique_name: &str) {
        // mpris2 library must have a timeout, but since this function calls it in a loop it
        // doesn't really matter what limit we set.
        const LOOP_INTERVAL_MS: u32 = 1000;
        let start = Instant::now();

        loop {
            for message in self.connection
                .incoming(LOOP_INTERVAL_MS)
                .filter(PooledConnection::is_watched_message)
            {
                self.process_message(&message);
                if self.is_bus_updated_after(unique_name, &start) {
                    return;
                }
            }
        }
    }

    /// Returns true if a given D-Bus Message is a MPRIS2 event (`ProeprtiesChanged` or `Seeked`
    /// delivered to a MPRIS2 path).
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

        false
    }

    /// Takes a message and updates the latest update time of the sender bus.
    fn process_message(&self, message: &Message) {
        message
            .sender()
            .map(|unique_name| self.mark_bus_as_updated((*unique_name).to_owned()));
    }

    fn mark_bus_as_updated<S: Into<String>>(&self, bus_name: S) {
        self.last_event
            .borrow_mut()
            .insert(bus_name.into(), Instant::now());
    }
}

impl From<Connection> for PooledConnection {
    fn from(connection: Connection) -> Self {
        PooledConnection::new(connection)
    }
}
