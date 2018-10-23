use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use dbus::{BusName, ConnPath, Connection, Message, Path};

use extensions::DurationExtensions;
use player::MPRIS2_PATH;

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
    events: RefCell<HashMap<String, Vec<MprisEvent>>>,
}

const GET_NAME_OWNER_TIMEOUT: i32 = 100; // ms
const NAME_HAS_OWNER_TIMEOUT: i32 = 100; // ms

impl PooledConnection {
    pub(crate) fn new(connection: Connection) -> Self {
        // Subscribe to events that relate to players. See `MprisMessage` below for details.
        let _ = connection.add_match(
            "interface='org.freedesktop.DBus.Properties',member='PropertiesChanged',path='/org/mpris/MediaPlayer2'",
        );
        let _ = connection.add_match(
            "interface='org.mpris.MediaPlayer2.Player',member='Seeked',path='/org/mpris/MediaPlayer2'",
        );
        let _ = connection.add_match(
            "interface='org.mpris.MediaPlayer2.TrackList',path='/org/mpris/MediaPlayer2'",
        );
        let _ = connection.add_match(
            "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged'",
        );
        PooledConnection {
            connection,
            events: RefCell::new(HashMap::new()),
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

    /// Returns `true` if the given bus name has any pending events waiting to be processed.
    ///
    /// If you want to actually act on the messages, use `pending_events`.
    pub(crate) fn has_pending_events(&self, bus_name: &str) -> bool {
        self.events
            .try_borrow()
            .ok()
            .map(|map| map.contains_key(bus_name))
            .unwrap_or(false)
    }

    /// Removes all pending events from a bus' queue and returns them.
    ///
    /// If you want to non-destructively check if a bus has anything queued, use
    /// `has_pending_events`.
    pub(crate) fn pending_events(&self, bus_name: &str) -> Vec<MprisEvent> {
        self.events
            .try_borrow_mut()
            .ok()
            .and_then(|mut events| events.remove(bus_name))
            .unwrap_or_default()
    }

    /// Process events in a blocking fashion until the deadline/timebox `Duration` runs out.
    pub(crate) fn process_events_blocking_for(&self, duration: Duration) {
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
                .flat_map(MprisMessage::try_parse)
                .next()
            {
                self.process_message(message);
            }
        }
    }

    /// Process events in a blocking fashion until any new event is found.
    pub(crate) fn process_events_blocking_until_received(&self) {
        // Loop will repeat every <internal> milliseconds, just waiting for new events to appear.
        let loop_interval = 5000; // ms
        loop {
            if let Some(message) = self
                .connection
                .incoming(loop_interval)
                .flat_map(MprisMessage::try_parse)
                .next()
            {
                self.process_message(message);
                return;
            }
        }
    }

    /// Takes a message and processes it appropriately. Returns the affected bus name, and a borrow
    /// to the generated MprisEvent, if applicable.
    fn process_message(&self, message: MprisMessage) {
        match message {
            MprisMessage::NameOwnerChanged {
                new_owner,
                old_owner,
            } => {
                // If `new_owner` is empty, then the client has quit.
                if new_owner.is_empty() {
                    // Clear out existing events, if any. Then add a "PlayerQuit" event on the
                    // queue.
                    let mut events = self.events.borrow_mut();
                    events.insert(old_owner, vec![MprisEvent::PlayerQuit]);
                } else {
                    // Just changed names for some reason. Migrate events that exist on the queue.
                    self.migrate_events(old_owner, new_owner);
                    // TODO: Make the player change it's unique name to the new name too.
                }
            }
            MprisMessage::PlayerPropertiesChanged { unique_name } => {
                let mut events = self.events.borrow_mut();
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::PlayerPropertiesChanged);
            }
            MprisMessage::Seeked {
                unique_name,
                position_in_us,
            } => {
                let mut events = self.events.borrow_mut();
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::Seeked { position_in_us });
            }
        }
    }

    fn migrate_events(&self, old_name: String, new_name: String) {
        let mut events = match self.events.try_borrow_mut() {
            Ok(val) => val,
            Err(_) => return,
        };

        if let Some(mut old_events) = events.remove(&old_name) {
            if events.contains_key(&new_name) {
                // Append the new events to the end of the old events and place them in the new
                // queue.
                old_events.append(&mut events.remove(&new_name).unwrap());
                events.insert(new_name, old_events);
            } else {
                // Move the queue over.
                events.insert(new_name, old_events);
            }
        }
    }
}

impl From<Connection> for PooledConnection {
    fn from(connection: Connection) -> Self {
        PooledConnection::new(connection)
    }
}

/// Event that a Player / ProgressTracker / Event iterator should react on. These are read via the
/// bus and placed on queues for each player. When a component asks for pending events of a player
/// they will be returned in the same order as they were emitted in.
#[derive(Debug)]
pub(crate) enum MprisEvent {
    PlayerQuit,
    PlayerPropertiesChanged,
    Seeked { position_in_us: u64 },
}

/// Easier to use representation of supported D-Bus messages.
#[derive(Debug)]
pub(crate) enum MprisMessage {
    NameOwnerChanged {
        new_owner: String,
        old_owner: String,
    },
    PlayerPropertiesChanged {
        unique_name: String,
    },
    Seeked {
        unique_name: String,
        position_in_us: u64,
    },
}

impl MprisMessage {
    /// Tries to convert the provided D-Bus message into a MprisMessage; returns None if the
    /// message was not supported.
    fn try_parse(message: Message) -> Option<Self> {
        MprisMessage::try_parse_name_owner_changed(&message)
            .or_else(|| MprisMessage::try_parse_player_properties_changed(&message))
            .or_else(|| MprisMessage::try_parse_seeked(&message))
    }

    /// Return a MprisMessage::NameOwnerChanged if the provided D-Bus message is a
    /// org.freedesktop.DBus NameOwnerChanged message.
    fn try_parse_name_owner_changed(message: &Message) -> Option<Self> {
        match (message.sender(), message.member()) {
            (Some(ref sender), Some(ref member))
                if &**sender == "org.freedesktop.DBus" && &**member == "NameOwnerChanged" =>
            {
                let mut iter = message.iter_init();
                let name: String = iter.read().ok()?;

                if !name.starts_with("org.mpris.") {
                    return None;
                }
                let old_owner: String = iter.read().ok()?;
                let new_owner: String = iter.read().ok()?;
                Some(MprisMessage::NameOwnerChanged {
                    new_owner,
                    old_owner,
                })
            }
            _ => None,
        }
    }

    /// Return a MprisMessage::PlayerPropertiesChanged if the provided D-Bus message is a
    /// PropertiesChanged signal on the MPRIS2 path.
    fn try_parse_player_properties_changed(message: &Message) -> Option<Self> {
        match (message.path(), message.member()) {
            (Some(ref path), Some(ref member))
                if &**path == MPRIS2_PATH && &**member == "PropertiesChanged" =>
            {
                message
                    .sender()
                    .map(|unique_name| MprisMessage::PlayerPropertiesChanged {
                        unique_name: unique_name.to_string(),
                    })
            }
            _ => None,
        }
    }

    /// Return a MprisMessage::Seeked if the provided D-Bus message is a Seeked signal on the
    /// MPRIS2 path.
    fn try_parse_seeked(message: &Message) -> Option<Self> {
        match (message.sender(), message.member()) {
            (Some(ref path), Some(ref member))
                if &**path == MPRIS2_PATH && &**member == "Seeked" =>
            {
                let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
                let mut iter = message.iter_init();
                let position_in_us: u64 = iter.read().ok()?;

                Some(MprisMessage::Seeked {
                    unique_name,
                    position_in_us,
                })
            }
            _ => None,
        }
    }
}
