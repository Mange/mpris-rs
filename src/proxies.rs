use std::collections::HashMap;

use zbus::dbus_proxy;
use zbus::zvariant::Value;

#[dbus_proxy(
    default_service = "org.freedesktop.DBus",
    interface = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus"
)]
pub(crate) trait DBus {
    fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[dbus_proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub(crate) trait MediaPlayer2 {
    #[dbus_proxy(property)]
    fn identity(&self) -> zbus::Result<String>;
}

impl MediaPlayer2Proxy<'_> {
    pub fn bus_name(&self) -> &str {
        self.inner().destination().as_str()
    }
}

#[dbus_proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub(crate) trait Player {
    #[dbus_proxy(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, Value>>;
}
