use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use dbus::{BusName, ConnPath, Connection, Member, Message, Path};

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
        // Subscribe to events that relate to players. See `is_watched_message` below for more
        // details.
        let _ = connection.add_match(
            "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'",
        );
        let _ = connection.add_match(
            "interface='org.mpris.MediaPlayer2.Player',member='Seeked',path='/org/mpris/MediaPlayer2'",
        );
        let _ = connection.add_match(
            "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged'",
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
                .map(|d| DurationExtensions::as_millis(&d))
                .unwrap_or(0);
            // Don't bother if we have very little time left
            if ms_left < 2 {
                break;
            }
            if let Some(message) = self
                .connection
                .incoming(ms_left as u32)
                .filter(PooledConnection::is_watched_message)
                .next()
            {
                self.process_message(&message);
            }
        }
    }

    /// Block until a MPRIS2 event for the given unique bus name is detected, or the bus disappears
    /// (the program exits).
    ///
    /// Events for other buses will also be recorded, but the method will not return until a
    /// matching one has been found or the bus disappears.
    pub(crate) fn process_events_blocking_until_dirty(&self, unique_name: &str) {
        // mpris2 library must have a timeout, but since this function calls it in a loop it
        // doesn't really matter what limit we set.
        const LOOP_INTERVAL_MS: u32 = 1000;
        let start = Instant::now();

        loop {
            for message in self
                .connection
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

    /// Returns true if a given D-Bus Message is a signal that is interesting for this library.
    ///   1. MPRIS2 "Seeked" signal.
    ///   2. D-Bus "PropertiesChanged" signal on a MPRIS2 path.
    ///   3. D-Bus "NameOwnerChanged" signal, to detect when players disappear.
    ///
    /// Since the connection given to the PooledConnection could already have subscriptions that
    /// are not listed in this file, it is important that messages are manually filtered before
    /// acting on them.
    fn is_watched_message(message: &Message) -> bool {
        if message.sender() == Some(BusName::from("org.freedesktop.DBus")) {
            message.member() == Some(Member::from("NameOwnerChanged"))
        } else if message.path() == Some(Path::from(MPRIS2_PATH)) {
            message.member() == Some(Member::from("PropertiesChanged"))
                || message.member() == Some(Member::from("Seeked"))
        } else {
            false
        }
    }

    /// Takes a message and updates the latest update time of the sender bus.
    fn process_message(&self, message: &Message) {
        if message.member() == Some(Member::from("NameOwnerChanged")) {
            let _ = self.process_name_owner_changed_event(message);
        } else {
            // Process mpris signal
            message
                .sender()
                .map(|unique_name| self.mark_bus_as_updated((*unique_name).to_owned()));
        }
    }

    fn process_name_owner_changed_event(
        &self,
        message: &Message,
    ) -> Result<(), ::dbus::arg::TypeMismatchError> {
        let mut iter = message.iter_init();
        let name: String = iter.read()?;

        if name.starts_with("org.mpris.") {
            let old_name: String = iter.read()?;
            let new_name: String = iter.read()?;

            // If "new_name" is empty, then the bus disappeared. "Wake" any potentially waiting
            // loop up.
            if new_name.is_empty() {
                self.mark_bus_as_updated(old_name);
            }
        }

        Ok(())
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
