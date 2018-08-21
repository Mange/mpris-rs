extern crate dbus;

use dbus::arg::{ArgType, RefArg, Variant};
use std::collections::HashMap;

/// Holds a dynamically-typed metadata value.
///
/// You will need to type-check this at runtime in order to use the value.
#[derive(Debug, PartialEq, Clone, EnumKind, is_enum_variant, FromVariants)]
#[enum_kind(ValueKind)]
pub enum Value {
    /// Value is a string.
    String(String),

    /// Value is a 16-bit integer.
    I16(i16),

    /// Value is a 32-bit integer.
    I32(i32),

    /// Value is a 64-bit integer.
    I64(i64),

    /// Value is an unsigned 8-bit integer.
    U8(u8),

    /// Value is an unsigned 16-bit integer.
    U16(u16),

    /// Value is an unsigned 32-bit integer.
    U32(u32),

    /// Value is an unsigned 64-bit integer.
    U64(u64),

    /// Value is a 64-bit float.
    F64(f64),

    /// Value is a boolean.
    Bool(bool),

    /// Value is an array of other values.
    Array(Vec<Value>),

    /// Value is a map of other values.
    Map(HashMap<String, Value>),

    /// Unsupported value type.
    #[from_variants(skip)]
    Unsupported,
}

impl Value {
    pub(crate) fn from_variant(variant: Variant<Box<RefArg + 'static>>) -> Value {
        Value::from_ref_arg(&variant.0)
    }

    pub(crate) fn from_ref_arg(ref_arg: &RefArg) -> Value {
        match ref_arg.arg_type() {
            ArgType::Array => ref_arg
                .as_iter()
                .map(|iter| Value::Array(iter.map(Value::from_ref_arg).collect())),
            ArgType::Boolean => ref_arg.as_u64().map(|n| Value::Bool(n == 1)),
            ArgType::Byte => ref_arg.as_u64().map(|n| Value::U8(n as u8)),
            ArgType::Double => ref_arg.as_f64().map(Value::from),
            ArgType::Int16 => ref_arg.as_i64().map(|n| Value::I16(n as i16)),
            ArgType::Int32 => ref_arg.as_i64().map(|n| Value::I32(n as i32)),
            ArgType::Int64 => ref_arg.as_i64().map(Value::I64),
            ArgType::String => ref_arg.as_str().map(Value::from),
            ArgType::UInt16 => ref_arg.as_u64().map(|n| Value::U16(n as u16)),
            ArgType::UInt32 => ref_arg.as_u64().map(|n| Value::U32(n as u32)),
            ArgType::UInt64 => ref_arg.as_u64().map(Value::U64),
            _ => None,
        }.unwrap_or(Value::Unsupported)
    }

    /// Returns a simple enum representing the type of value that this value holds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate mpris;
    /// # extern crate dbus;
    /// # use mpris::Metadata;
    /// # fn main() {
    /// # let metadata = Metadata::new("1234");
    /// # let key_name = "foo";
    /// use mpris::MetadataValueKind;
    /// if let Some(value) = metadata.get(key_name) {
    ///     match value.kind() {
    ///       MetadataValueKind::String => println!("{} is a string", key_name),
    ///       MetadataValueKind::I16 |
    ///       MetadataValueKind::I32 |
    ///       MetadataValueKind::I64 |
    ///       MetadataValueKind::U8 |
    ///       MetadataValueKind::U16 |
    ///       MetadataValueKind::U32 |
    ///       MetadataValueKind::U64 => println!("{} is an integer", key_name),
    ///       MetadataValueKind::F64 => println!("{} is a float", key_name),
    ///       MetadataValueKind::Bool => println!("{} is a boolean", key_name),
    ///       MetadataValueKind::Array => println!("{} is an array", key_name),
    ///       MetadataValueKind::Map => println!("{} is a map", key_name),
    ///       MetadataValueKind::Unsupported => println!("{} is not a supported type", key_name),
    ///     }
    /// } else {
    ///     println!("Metadata does not have a {} key", key_name);
    /// }
    /// # }
    /// ```
    pub fn kind(&self) -> ValueKind {
        ValueKind::from(self)
    }
}

include!("value_conversions.rs");

impl<'a> From<&'a str> for Value {
    fn from(string: &'a str) -> Value {
        Value::String(String::from(string))
    }
}

impl dbus::arg::Arg for Value {
    const ARG_TYPE: ArgType = ArgType::Variant;
    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from_slice(b"v\0").unwrap()
    }
}

impl<'a> dbus::arg::Get<'a> for Value {
    fn get(i: &mut dbus::arg::Iter) -> Option<Self> {
        use dbus::arg::ArgType;

        let arg_type = i.arg_type();
        // Trying to calculate signature of an invalid arg will panic, so abort early.
        if let ArgType::Invalid = arg_type {
            return None;
        }
        let signature = i.signature();

        match arg_type {
            // Hashes in DBus are arrays of Dict pairs ({string, variant})
            ArgType::Array if *signature == *"a{sv}" => {
                i.get::<HashMap<String, Value>>().map(Value::from)
            }
            ArgType::Array => i.get::<Vec<Value>>().map(Value::from),
            ArgType::Boolean => i.get::<bool>().map(Value::from),
            ArgType::Byte => i.get::<u8>().map(Value::from),
            ArgType::Double => i.get::<f64>().map(Value::from),
            ArgType::Int16 => i.get::<i16>().map(Value::from),
            ArgType::Int32 => i.get::<i32>().map(Value::from),
            ArgType::Int64 => i.get::<i64>().map(Value::from),
            ArgType::String => i.get::<String>().map(Value::from),
            ArgType::UInt16 => i.get::<u16>().map(Value::from),
            ArgType::UInt32 => i.get::<u32>().map(Value::from),
            ArgType::UInt64 => i.get::<u64>().map(Value::from),
            ArgType::Variant => i.recurse(ArgType::Variant).and_then(|mut iter| iter.get()),
            ArgType::Invalid => unreachable!("Early return at the top of the method"),
            ArgType::DictEntry
            | ArgType::UnixFd
            | ArgType::Signature
            | ArgType::ObjectPath
            | ArgType::Struct => Some(Value::Unsupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbus::arg::Append;
    use dbus::{BusType, Connection, ConnectionItem, Message};

    fn send_values_over_dbus<F>(appender: F) -> Message
    where
        F: FnOnce(Message) -> Message,
    {
        //
        // Open a connection, send a message to it and then read the message back again.
        //
        let connection = Connection::get_private(BusType::Session)
            .expect("Could not open a D-Bus session connection");
        connection
            .register_object_path("/hello")
            .expect("Could not register object path");
        let send_message = Message::new_method_call(
            &connection.unique_name(),
            "/hello",
            "com.example.hello",
            "Hello",
        ).expect("Could not create message");

        let send_message = appender(send_message);

        connection
            .send(send_message)
            .expect("Could not send message to myself");

        for item in connection.iter(200) {
            if let ConnectionItem::MethodCall(received_message) = item {
                return received_message;
            }
        }

        panic!("Did not find a message on the bus");
    }

    fn send_value_over_dbus<T: Append>(value: T) -> Message {
        send_values_over_dbus(|message| message.append1(value))
    }

    #[test]
    fn it_supports_strings() {
        let message = send_value_over_dbus("Hello world!");

        let string: Value = message.get1().unwrap();
        assert!(string.is_string());
        assert_eq!(string.as_str(), Some("Hello world!"));
    }

    #[test]
    fn it_supports_unsigned_integers() {
        let message = send_values_over_dbus(|input| input.append3(1u8, 2u16, 3u32).append1(4u64));

        let mut values = message.iter_init();
        let eight: Value = values.read().unwrap();
        let sixteen: Value = values.read().unwrap();
        let thirtytwo: Value = values.read().unwrap();
        let sixtyfour: Value = values.read().unwrap();

        assert_eq!(Value::U8(1), eight);
        assert_eq!(Value::U16(2), sixteen);
        assert_eq!(Value::U32(3), thirtytwo);
        assert_eq!(Value::U64(4), sixtyfour);
    }

    #[test]
    fn it_supports_signed_integers() {
        let message = send_values_over_dbus(|input| input.append3(1i16, 2i32, 3i64));

        let mut values = message.iter_init();
        let sixteen: Value = values.read().unwrap();
        let thirtytwo: Value = values.read().unwrap();
        let sixtyfour: Value = values.read().unwrap();

        assert_eq!(Value::I16(1), sixteen);
        assert_eq!(Value::I32(2), thirtytwo);
        assert_eq!(Value::I64(3), sixtyfour);
    }

    #[test]
    fn it_supports_floats() {
        let message = send_value_over_dbus(42.0f64);

        let float: Value = message.get1().unwrap();
        assert!(float.is_f64());
        assert_eq!(float.as_f64(), Some(42.0));
    }

    #[test]
    fn it_supports_booleans() {
        let message = send_value_over_dbus(true);

        let boolean: Value = message.get1().unwrap();
        assert!(boolean.is_bool());
        assert_eq!(boolean.as_bool(), Some(true));
    }

    #[test]
    fn it_supports_arrays_of_variants() {
        let input: Vec<Variant<Box<RefArg>>> = vec![
            Variant(Box::new(String::from("World"))),
            Variant(Box::new(42u8)),
        ];

        let expected = vec![Value::String("World".into()), Value::U8(42)];

        let message = send_value_over_dbus(input);

        let array: Value = message.get1().unwrap();
        assert!(array.is_array());
        assert_eq!(array.into_array(), Some(expected));
    }

    #[test]
    fn it_supports_arrays_of_strings() {
        let input: Vec<String> = vec!["Hello".into(), "World".into()];

        let expected: Vec<Value> = vec!["Hello".into(), "World".into()];

        let message = send_value_over_dbus(input);

        let array: Value = message.get1().unwrap();
        assert!(array.is_array());
        assert_eq!(array.into_array(), Some(expected));
    }

    #[test]
    fn it_supports_maps_of_variants() {
        let mut input: HashMap<String, Variant<Box<RefArg>>> = HashMap::new();
        input.insert(
            String::from("receiver"),
            Variant(Box::new(String::from("World"))),
        );
        input.insert(String::from("times"), Variant(Box::new(42u8)));

        let mut expected = HashMap::new();
        expected.insert(
            String::from("receiver"),
            Value::String(String::from("World")),
        );
        expected.insert(String::from("times"), Value::U8(42));

        let message = send_value_over_dbus(input);

        let hash: Value = message.get1().unwrap();
        assert!(hash.is_map());
        assert_eq!(hash.into_map(), Some(expected));
    }
}
