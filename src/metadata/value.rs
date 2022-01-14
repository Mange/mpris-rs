use dbus::arg::ArgType;
use derive_is_enum_variant::is_enum_variant;
use enum_kinds::EnumKind;
use from_variants::FromVariants;
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
    /// Returns a simple enum representing the type of value that this value holds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use mpris::Metadata;
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
    /// ```
    pub fn kind(&self) -> ValueKind {
        ValueKind::from(self)
    }

    /// Returns the value as a `Some(Vec<&str>)` if it is a `MetadataValue::Array`. Any elements
    /// that are not `MetadataValue::String` values will be ignored.
    pub fn as_str_array(&self) -> Option<Vec<&str>> {
        match *self {
            Value::Array(ref vec) => Some(vec.iter().flat_map(Value::as_str).collect()),
            _ => None,
        }
    }

    /// Returns the value as a `Some(u8)` if it is a `MetadataValue::U8`, or `None` otherwise.
    pub fn as_u8(&self) -> Option<u8> {
        match *self {
            Value::U8(val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(u16)` if it is an unsigned int smaller than or equal to u16,
    /// or `None` otherwise.
    pub fn as_u16(&self) -> Option<u16> {
        match *self {
            Value::U16(val) => Some(val),
            Value::U8(val) => Some(u16::from(val)),
            _ => None,
        }
    }

    /// Returns the value as a `Some(u32)` if it is an unsigned int smaller than or equal to u32,
    /// or `None` otherwise.
    pub fn as_u32(&self) -> Option<u32> {
        match *self {
            Value::U32(val) => Some(val),
            Value::U16(val) => Some(u32::from(val)),
            Value::U8(val) => Some(u32::from(val)),
            _ => None,
        }
    }

    /// Returns the value as a `Some(u64)` if it is an unsigned int smaller than or equal to u64,
    /// or `None` otherwise.
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            Value::U64(val) => Some(val),
            Value::U32(val) => Some(u64::from(val)),
            Value::U16(val) => Some(u64::from(val)),
            Value::U8(val) => Some(u64::from(val)),
            _ => None,
        }
    }

    /// Returns the value as a `Some(i16)` if it is a signed integer smaller than or equal to i16,
    /// or `None` otherwise.
    pub fn as_i16(&self) -> Option<i16> {
        match *self {
            Value::I16(val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(i32)` if it is a signed integer smaller than or equal to i32,
    /// or `None` otherwise.
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::I32(val) => Some(val),
            Value::I16(val) => Some(i32::from(val)),
            _ => None,
        }
    }

    /// Returns the value as a `Some(i64)` if it is a signed integer smaller than or equal to i64,
    /// or `None` otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::I64(val) => Some(val),
            Value::I32(val) => Some(i64::from(val)),
            Value::I16(val) => Some(i64::from(val)),
            _ => None,
        }
    }

    /// Returns the value as a `Some(f64)` if it is a `MetadataValue::F64`, or `None` otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::F64(val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(bool)` if it is a `MetadataValue::Bool`, or `None` otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Bool(val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(&str)` if it is a `MetadataValue::String`, or `None` otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(&String)` if it is a `MetadataValue::String`, or `None` otherwise.
    pub fn as_string(&self) -> Option<&String> {
        match *self {
            Value::String(ref val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(&HashMap<String, Value>)` if it is a `MetadataValue::Map`, or `None` otherwise.
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match *self {
            Value::Map(ref val) => Some(val),
            _ => None,
        }
    }

    /// Returns the value as a `Some(&Vec<Value>)` if it is a `MetadataValue::Array`, or `None` otherwise.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match *self {
            Value::Array(ref val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(u8)` if it is a `MetadataValue::U8`, or `None` otherwise.
    pub fn into_u8(self) -> Option<u8> {
        match self {
            Value::U8(val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(u16)` if it is an unsigned integer
    /// smaller than or equal to u16, or `None` otherwise.
    pub fn into_u16(self) -> Option<u16> {
        match self {
            Value::U16(val) => Some(val),
            Value::U8(val) => Some(u16::from(val)),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(u32)` if it is an unsigned integer
    /// smaller than or equal to u32, or `None` otherwise.
    pub fn into_u32(self) -> Option<u32> {
        match self {
            Value::U32(val) => Some(val),
            Value::U16(val) => Some(u32::from(val)),
            Value::U8(val) => Some(u32::from(val)),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(u64)` if it is an unsigned integer
    /// smaller than or equal to u64, or `None` otherwise.
    pub fn into_u64(self) -> Option<u64> {
        match self {
            Value::U64(val) => Some(val),
            Value::U32(val) => Some(u64::from(val)),
            Value::U16(val) => Some(u64::from(val)),
            Value::U8(val) => Some(u64::from(val)),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(i16)` if it is a signed integer
    /// smaller than or equal to i16, or `None` otherwise.
    pub fn into_i16(self) -> Option<i16> {
        match self {
            Value::I16(val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(i32)` if it is a signed integer
    /// smaller than or equal to i32, or `None` otherwise.
    pub fn into_i32(self) -> Option<i32> {
        match self {
            Value::I32(val) => Some(val),
            Value::I16(val) => Some(i32::from(val)),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(i64)` if it is a signed integer
    /// smaller than or equal to i64, or `None` otherwise.
    pub fn into_i64(self) -> Option<i64> {
        match self {
            Value::I64(val) => Some(val),
            Value::I32(val) => Some(i64::from(val)),
            Value::I16(val) => Some(i64::from(val)),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(f64)` if it is a
    /// `MetadataValue::F64`, or `None` otherwise.
    pub fn into_f64(self) -> Option<f64> {
        match self {
            Value::F64(val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(bool)` if it is a
    /// `MetadataValue::Bool`, or `None` otherwise.
    pub fn into_bool(self) -> Option<bool> {
        match self {
            Value::Bool(val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(String)` if it is a
    /// `MetadataValue::String`, or `None` otherwise.
    pub fn into_string(self) -> Option<String> {
        match self {
            Value::String(val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(HashMap<String, Value>)` if it is a
    /// `MetadataValue::Map`, or `None` otherwise.
    pub fn into_map(self) -> Option<HashMap<String, Value>> {
        match self {
            Value::Map(val) => Some(val),
            _ => None,
        }
    }

    /// Consumes `self` and returns the inner value as a `Some(Vec<Value>)` if it is a
    /// `MetadataValue::Array`, or `None` otherwise.
    pub fn into_array(self) -> Option<Vec<Value>> {
        match self {
            Value::Array(val) => Some(val),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for Value {
    fn from(string: &'a str) -> Value {
        Value::String(String::from(string))
    }
}

impl dbus::arg::Arg for Value {
    const ARG_TYPE: ArgType = ArgType::Variant;
    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from_slice("v").unwrap()
    }
}

impl<'a> dbus::arg::Get<'a> for Value {
    fn get(i: &mut dbus::arg::Iter<'_>) -> Option<Self> {
        let arg_type = i.arg_type();
        // Trying to calculate signature of an invalid arg will panic, so abort early.
        if let ArgType::Invalid = arg_type {
            return None;
        }
        let signature = i.signature();

        match arg_type {
            // Hashes in DBus are arrays of Dict pairs ({string, variant})
            ArgType::Array if *signature == *"a{sv}" => {
                i.get::<HashMap<String, Value>>().map(Value::Map)
            }
            ArgType::Array => i.get::<Vec<Value>>().map(Value::Array),
            ArgType::Boolean => i.get::<bool>().map(Value::Bool),
            ArgType::Byte => i.get::<u8>().map(Value::U8),
            ArgType::Double => i.get::<f64>().map(Value::F64),
            ArgType::Int16 => i.get::<i16>().map(Value::I16),
            ArgType::Int32 => i.get::<i32>().map(Value::I32),
            ArgType::Int64 => i.get::<i64>().map(Value::I64),
            ArgType::String => i.get::<String>().map(Value::String),
            ArgType::UInt16 => i.get::<u16>().map(Value::U16),
            ArgType::UInt32 => i.get::<u32>().map(Value::U32),
            ArgType::UInt64 => i.get::<u64>().map(Value::U64),
            ArgType::Variant => i.recurse(ArgType::Variant).and_then(|mut iter| iter.get()),
            ArgType::Invalid => unreachable!("Early return at the top of the method"),
            ArgType::ObjectPath => i
                .get::<dbus::Path<'_>>()
                .map(|p| Value::String(p.to_string())),
            ArgType::DictEntry | ArgType::UnixFd | ArgType::Signature | ArgType::Struct => {
                Some(Value::Unsupported)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbus::arg::{Append, RefArg, Variant};
    use dbus::ffidisp::{BusType, Connection, ConnectionItem};
    use dbus::Message;

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
        )
        .expect("Could not create message");

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
    fn it_supports_object_paths_as_strings() {
        let message = send_value_over_dbus(dbus::Path::from("/hello/world"));

        let string: Value = message.get1().unwrap();
        assert!(string.is_string());
        assert_eq!(string.as_str(), Some("/hello/world"));
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
        let input: Vec<Variant<Box<dyn RefArg>>> = vec![
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
        let mut input: HashMap<String, Variant<Box<dyn RefArg>>> = HashMap::new();
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
