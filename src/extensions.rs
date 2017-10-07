use errors::*;

use dbus::MessageItem;

pub trait MessageItemExtensions {
    fn type_name(&self) -> &'static str;
    fn as_string(self, name: &'static str) -> Result<String>;
    fn as_bool(self, name: &'static str) -> Result<bool>;
    fn as_array(self, name: &'static str) -> Result<Vec<MessageItem>>;
    fn as_string_array(self, name: &'static str) -> Result<Vec<String>>;
    fn as_dict_array(self, name: &'static str) -> Result<Vec<(MessageItem, MessageItem)>>;
}

impl MessageItemExtensions for MessageItem {
    fn type_name(&self) -> &'static str {
        use dbus::MessageItem::*;

        match *self {
            Array(..) => "Array",
            Struct(..) => "Struct",
            Variant(..) => "Variant",
            DictEntry(..) => "DictEntry",
            ObjectPath(..) => "ObjectPath",
            Str(..) => "Str",
            Bool(..) => "Bool",
            Byte(..) => "Byte",
            Int16(..) => "Int16",
            Int32(..) => "Int32",
            Int64(..) => "Int64",
            UInt16(..) => "UInt16",
            UInt32(..) => "UInt32",
            UInt64(..) => "UInt64",
            Double(..) => "Double",
            UnixFd(..) => "UnixFd",
        }
    }

    fn as_string(self, name: &'static str) -> Result<String> {
        self.inner::<&str>().map(String::from).map_err(|_| {
            format!(
                "Expected {} to be a String but was a {}",
                name,
                self.type_name()
            ).into()
        })
    }

    fn as_bool(self, name: &'static str) -> Result<bool> {
        self.inner::<bool>().map_err(|_| {
            format!(
                "Expected {} to be a bool but was a {}",
                name,
                self.type_name()
            ).into()
        })
    }

    fn as_array(self, name: &'static str) -> Result<Vec<MessageItem>> {
        match self {
            MessageItem::Array(values, _) => Ok(values),
            _ => {
                bail!(
                    "Expected {} to be Array but was a {}",
                    name,
                    self.type_name()
                )
            }
        }
    }

    fn as_string_array(self, name: &'static str) -> Result<Vec<String>> {
        let array = self.as_array(name)?;
        Ok(
            array
                .into_iter()
                .filter_map(|v| v.as_string("array item").ok())
                .collect(),
        )
    }

    fn as_dict_array(self, name: &'static str) -> Result<Vec<(MessageItem, MessageItem)>> {
        let array = self.as_array(name)?;
        let tuple_array = array
            .into_iter()
            .filter_map(|entry| match entry {
                MessageItem::DictEntry(key, value) => {
                    match *value {
                        MessageItem::Variant(nested) => Some((*key, *nested)),
                        _ => Some((*key, *value)),
                    }
                }
                _ => None,
            })
            .collect();
        Ok(tuple_array)
    }
}
