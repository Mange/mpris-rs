use dbus::{Connection, BusName, Props};

use prelude::*;
use metadata::Metadata;

pub struct Player<'conn> {
    connection: &'conn Connection,
    bus_name: BusName<'conn>,
}

impl<'conn> Player<'conn> {
    pub fn new<B>(connection: &'conn Connection, bus_name: B) -> Player<'conn>
    where
        B: Into<BusName<'conn>>,
    {
        Player {
            connection: connection,
            bus_name: bus_name.into(),
        }
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
}
