#[macro_use]
extern crate error_chain;
extern crate dbus;

pub mod errors {
    error_chain!{
        foreign_links {
            DBusError(::dbus::Error);
        }

        errors {
            NoPlayerFound {
                description("No player found")
                display("Could not find a compatible MPRIS2 player running right now.")
            }
            TrackIdMissing {
                description("track_id missing")
                display("mpris:trackid not present in metadata")
            }
            DBusCallError(message: String) {
                description("DBus call failed")
                display("DBus call failed: {}", message)
            }
        }
    }
}

mod extensions;
mod player;
mod find;
mod metadata;

mod prelude {
    pub use extensions::*;
    pub use errors::*;
}

pub use player::Player;
pub use metadata::Metadata;
pub use find::PlayerFinder;
