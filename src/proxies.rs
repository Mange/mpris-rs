use zbus::dbus_proxy;

#[dbus_proxy]
pub(crate) trait DBus {
    fn list_names(&self) -> zbus::Result<Vec<String>>;
}

#[dbus_proxy(interface = "org.freedesktop.DBus.Properties")]
pub(crate) trait MprisPlayer {
    #[property]
    fn identity(&self) -> zbus::Result<String>;
}
