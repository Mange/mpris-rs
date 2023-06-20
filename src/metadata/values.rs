use zbus::zvariant::Value;

/*
* Subset of DBus data types that are commonly used in MPRIS metadata, and a boolean variant as it
* seems likely to be used in some custom metadata.
*
* See https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
*/
#[derive(Debug)]
pub enum MetadataValue {
    Boolean(bool),
    Float(f64),
    SignedInt(i64),
    UnsignedInt(u64),
    String(String),
    Strings(Vec<String>),
    Unsupported,
}

impl MetadataValue {
    pub fn into_string(self) -> Option<String> {
        if let MetadataValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn into_nonempty_string(self) -> Option<String> {
        self.into_string()
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
    }

    pub fn into_i64(self) -> Option<i64> {
        match self {
            MetadataValue::SignedInt(i) => Some(i),
            MetadataValue::UnsignedInt(i) => Some(0i64.saturating_add_unsigned(i)),
            _ => None,
        }
    }

    pub fn into_u64(self) -> Option<u64> {
        match self {
            MetadataValue::SignedInt(i) if i < 0 => Some(0),
            MetadataValue::SignedInt(i) => Some(i as u64),
            MetadataValue::UnsignedInt(i) => Some(i),
            _ => None,
        }
    }

    pub fn into_float(self) -> Option<f64> {
        if let MetadataValue::Float(f) = self {
            Some(f)
        } else {
            None
        }
    }

    pub fn into_strings(self) -> Option<Vec<String>> {
        match self {
            MetadataValue::Strings(v) => Some(v),
            MetadataValue::String(s) => Some(vec![s]),
            _ => None,
        }
    }
}

impl<'a> From<Value<'a>> for MetadataValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Bool(v) => MetadataValue::Boolean(v),
            Value::I16(v) => MetadataValue::SignedInt(v as i64),
            Value::I32(v) => MetadataValue::SignedInt(v as i64),
            Value::I64(v) => MetadataValue::SignedInt(v as i64),
            Value::U16(v) => MetadataValue::UnsignedInt(v as u64),
            Value::U32(v) => MetadataValue::UnsignedInt(v as u64),
            Value::U64(v) => MetadataValue::UnsignedInt(v as u64),
            Value::U8(v) => MetadataValue::UnsignedInt(v as u64),

            Value::F64(v) => MetadataValue::Float(v),

            Value::Str(v) => MetadataValue::String(v.to_string()),
            Value::Signature(v) => MetadataValue::String(v.to_string()),
            Value::ObjectPath(v) => MetadataValue::String(v.to_string()),

            Value::Array(a) if a.full_signature() == "as" => {
                let mut strings = Vec::with_capacity(a.len());
                for v in a.into_iter() {
                    if let Value::Str(s) = v {
                        strings.push(s.to_string());
                    }
                }
                MetadataValue::Strings(strings)
            }

            Value::Value(v) => MetadataValue::from(*v),

            Value::Array(_) => MetadataValue::Unsupported,
            Value::Dict(_) => MetadataValue::Unsupported,
            Value::Structure(_) => MetadataValue::Unsupported,
            Value::Fd(_) => MetadataValue::Unsupported,
        }
    }
}

#[test]
fn test_signed_integer_casting() {
    assert_eq!(MetadataValue::SignedInt(42).into_i64(), Some(42));
    assert_eq!(MetadataValue::SignedInt(-42).into_i64(), Some(-42));
    assert_eq!(MetadataValue::UnsignedInt(42).into_i64(), Some(42));
    assert_eq!(MetadataValue::Boolean(true).into_i64(), None);

    assert_eq!(
        MetadataValue::UnsignedInt(u64::MAX).into_i64(),
        Some(i64::MAX)
    );
}

#[test]
fn test_unsigned_integer_casting() {
    assert_eq!(MetadataValue::SignedInt(42).into_u64(), Some(42));
    assert_eq!(MetadataValue::SignedInt(-42).into_u64(), Some(0));
    assert_eq!(MetadataValue::UnsignedInt(42).into_u64(), Some(42));
    assert_eq!(MetadataValue::Boolean(true).into_u64(), None);

    assert_eq!(
        MetadataValue::SignedInt(i64::MAX).into_u64(),
        Some(i64::MAX as u64)
    );

    assert_eq!(MetadataValue::SignedInt(i64::MIN).into_u64(), Some(0));

    assert_eq!(
        MetadataValue::UnsignedInt(u64::MAX).into_u64(),
        Some(u64::MAX)
    );
}
