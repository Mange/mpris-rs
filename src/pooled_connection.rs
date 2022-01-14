use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use dbus::ffidisp::{ConnPath, Connection};
use dbus::strings::{BusName, Path};
use dbus::Message;

use crate::extensions::DurationExtensions;
use crate::metadata::{Metadata, Value};
use crate::player::MPRIS2_PATH;
use crate::track_list::TrackID;

#[derive(Debug)]
pub(crate) struct PooledConnection {
    connection: Connection,
    events: RefCell<HashMap<String, Vec<MprisEvent>>>,
}

const GET_NAME_OWNER_TIMEOUT: i32 = 100; // ms
const NAME_HAS_OWNER_TIMEOUT: i32 = 100; // ms

impl PooledConnection {
    pub(crate) fn new(connection: Connection) -> Self {
        // Subscribe to events that relate to players. See [`MprisMessage`] below for details.
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
    ) -> ConnPath<'_, &'a Connection> {
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
        )
        .unwrap()
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
        )
        .unwrap()
        .append1(bus_name.into());

        self.connection
            .send_with_reply_and_block(name_has_owner, NAME_HAS_OWNER_TIMEOUT)
            .ok()
            .and_then(|reply| reply.get1())
    }

    /// Returns [`true`] if the given bus name has any pending events waiting to be processed.
    ///
    /// If you want to actually act on the messages, use [`pending_events`](Self::pending_events).
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
    /// [`has_pending_events`](Self::has_pending_events).
    pub(crate) fn pending_events(&self, bus_name: &str) -> Vec<MprisEvent> {
        self.events
            .try_borrow_mut()
            .ok()
            .and_then(|mut events| events.remove(bus_name))
            .unwrap_or_default()
    }

    /// Process events in a blocking fashion until the deadline/timebox [`Duration`] runs out.
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
    /// to the generated [`MprisEvent`], if applicable.
    fn process_message(&self, message: MprisMessage) {
        let mut events = match self.events.try_borrow_mut() {
            Ok(val) => val,
            Err(_) => {
                // Drop the message. This is a better evil than triggering a panic inside a library
                // like this.
                return;
            }
        };

        match message {
            MprisMessage::NameOwnerChanged {
                new_owner,
                old_owner,
            } => {
                // If `new_owner` is empty, then the client has quit.
                if new_owner.is_empty() {
                    // Clear out existing events, if any. Then add a "PlayerQuit" event on the
                    // queue.
                    events.insert(old_owner, vec![MprisEvent::PlayerQuit]);
                }
            }
            MprisMessage::PlayerPropertiesChanged { unique_name } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::PlayerPropertiesChanged);
            }
            MprisMessage::Seeked {
                unique_name,
                position_in_us,
            } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::Seeked { position_in_us });
            }
            MprisMessage::TrackListPropertiesChanged { unique_name } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::TrackListPropertiesChanged);
            }
            MprisMessage::TrackListReplaced {
                unique_name, ids, ..
            } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::TrackListReplaced {
                        ids: ids.into_iter().map(TrackID::from).collect(),
                    });
            }
            MprisMessage::TrackAdded {
                unique_name,
                after_id,
                metadata,
            } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::TrackAdded {
                        after_id,
                        metadata: Metadata::from(metadata),
                    });
            }
            MprisMessage::TrackRemoved { unique_name, id } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::TrackRemoved { id });
            }
            MprisMessage::TrackMetadataChanged {
                unique_name,
                old_id,
                metadata,
            } => {
                events
                    .entry(unique_name)
                    .or_default()
                    .push(MprisEvent::TrackMetadataChanged {
                        old_id,
                        metadata: Metadata::from(metadata),
                    });
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
    Seeked {
        position_in_us: u64,
    },
    TrackListPropertiesChanged,
    TrackListReplaced {
        ids: Vec<TrackID>,
    },
    TrackAdded {
        after_id: TrackID,
        metadata: Metadata,
    },
    TrackRemoved {
        id: TrackID,
    },
    TrackMetadataChanged {
        old_id: TrackID,
        metadata: Metadata,
    },
}

/// Easier to use representation of supported [`D-Bus message`](Message).
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
    TrackListPropertiesChanged {
        unique_name: String,
    },
    TrackListReplaced {
        unique_name: String,
        ids: Vec<TrackID>,
        _current_id: TrackID,
    },
    TrackAdded {
        unique_name: String,
        after_id: TrackID,
        metadata: HashMap<String, Value>,
    },
    TrackRemoved {
        unique_name: String,
        id: TrackID,
    },
    TrackMetadataChanged {
        unique_name: String,
        old_id: TrackID,
        metadata: HashMap<String, Value>,
    },
}

impl MprisMessage {
    /// Tries to convert the provided [`D-Bus message`](Message) into a MprisMessage; returns [`None`] if the
    /// message was not supported.
    fn try_parse(message: Message) -> Option<Self> {
        MprisMessage::try_parse_name_owner_changed(&message)
            .or_else(|| MprisMessage::try_parse_mpris_signal(&message))
    }

    /// Return a [`MprisMessage::NameOwnerChanged`] if the provided D-Bus message is a
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

    fn try_parse_mpris_signal(message: &Message) -> Option<Self> {
        if let Some(ref path) = message.path() {
            if &**path == MPRIS2_PATH {
                let member = message
                    .member()
                    .map(|member| member.to_string())
                    .unwrap_or_else(String::default);
                return match member.as_ref() {
                    "PropertiesChanged" => try_parse_properties_changed(message),
                    "Seeked" => try_parse_seeked(message),
                    "TrackListReplaced" => try_parse_tracklist_replaced(message),
                    "TrackAdded" => try_parse_track_added(message),
                    "TrackRemoved" => try_parse_track_removed(message),
                    "TrackMetadataChanged" => try_parse_track_metadata_changed(message),
                    _ => None,
                };
            }
        }
        None
    }
}

fn try_parse_properties_changed(message: &Message) -> Option<MprisMessage> {
    let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
    let mut iter = message.iter_init();
    let interface_name: String = iter.read().ok()?;
    match interface_name.as_ref() {
        "org.mpris.MediaPlayer2.Player" => {
            Some(MprisMessage::PlayerPropertiesChanged { unique_name })
        }
        "org.mpris.MediaPlayer2.TrackList" => {
            Some(MprisMessage::TrackListPropertiesChanged { unique_name })
        }
        _ => None,
    }
}

fn try_parse_seeked(message: &Message) -> Option<MprisMessage> {
    let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
    let mut iter = message.iter_init();
    let position_in_us: u64 = iter.read().ok()?;

    Some(MprisMessage::Seeked {
        unique_name,
        position_in_us,
    })
}

fn try_parse_tracklist_replaced(message: &Message) -> Option<MprisMessage> {
    let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
    let mut iter = message.iter_init();
    let ids: Vec<Path<'_>> = iter.read().ok()?;
    let current_id: Path<'_> = iter.read().ok()?;

    Some(MprisMessage::TrackListReplaced {
        unique_name,
        ids: ids.into_iter().map(TrackID::from).collect(),
        _current_id: TrackID::from(current_id),
    })
}

fn try_parse_track_added(message: &Message) -> Option<MprisMessage> {
    let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
    let mut iter = message.iter_init();
    let metadata: HashMap<String, Value> = iter.read().ok()?;
    let after_id: Path<'_> = iter.read().ok()?;

    Some(MprisMessage::TrackAdded {
        unique_name,
        metadata,
        after_id: TrackID::from(after_id),
    })
}

fn try_parse_track_removed(message: &Message) -> Option<MprisMessage> {
    let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
    let mut iter = message.iter_init();
    let id: Path<'_> = iter.read().ok()?;

    Some(MprisMessage::TrackRemoved {
        unique_name,
        id: TrackID::from(id),
    })
}

fn try_parse_track_metadata_changed(message: &Message) -> Option<MprisMessage> {
    let unique_name = message.sender().map(|bus_name| bus_name.to_string())?;
    let mut iter = message.iter_init();
    let old_id: Path<'_> = iter.read().ok()?;
    let metadata: HashMap<String, Value> = iter.read().ok()?;

    Some(MprisMessage::TrackMetadataChanged {
        unique_name,
        old_id: TrackID::from(old_id),
        metadata,
    })
}
