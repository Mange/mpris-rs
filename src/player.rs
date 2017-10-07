use dbus::{Connection, BusName, Props};

use prelude::*;
use metadata::Metadata;

pub struct Player<'conn> {
    connection: &'conn Connection,
    bus_name: BusName<'conn>,
    identity: String,
}

impl<'conn> Player<'conn> {
    pub fn new<B>(connection: &'conn Connection, bus_name: B) -> Result<Player<'conn>>
    where
        B: Into<BusName<'conn>>,
    {
        let bus_name = bus_name.into();

        let props = Props::new(
            connection,
            bus_name.clone(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2",
            500,
        );

        let identity = props.get("Identity").map_err(|e| e.into()).and_then(|v| {
            v.as_string("Identity")
        })?;

        Ok(Player {
            connection: connection,
            bus_name: bus_name,
            identity: identity,
        })
    }

    pub fn bus_name(&self) -> &str {
        &self.bus_name
    }

    pub fn get_metadata(&self) -> Result<Metadata> {
        let props = Props::new(
            self.connection,
            self.bus_name.clone(),
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            500,
        );

        props.get("Metadata").map_err(|e| e.into()).and_then(|v| {
            Metadata::new_from_message_item(v)
        })
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }
}
