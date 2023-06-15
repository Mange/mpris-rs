use zbus::dbus_proxy;

#[dbus_proxy(
    default_service = "org.freedesktop.DBus",
    interface = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus"
)]
pub(crate) trait DBus {
    fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[dbus_proxy(
    interface = "org.freedesktop.DBus.Properties",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub(crate) trait MprisPlayer {
    async fn identity(&self) -> zbus::Result<String>;
}

impl MprisPlayerProxy<'_> {
    pub fn bus_name(&self) -> &str {
        self.inner().destination().as_str()
    }
}
