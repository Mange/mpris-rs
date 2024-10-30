use std::{
    ops::{Add, Div, Mul, Sub},
    time::Duration,
};

use crate::{errors::InvalidMprisDuration, metadata::MetadataValue};

const MAX: u64 = i64::MAX as u64;

/// A Mpris specific version of [`Duration`].
///
/// Contains a time duration in microseconds that's a non-negative [`i64`]. Technically the MPRIS
/// spec allows for time durations to be negative even though a song can't have a negative
/// length/position but this type doesn't allow them.
///
/// ## Creation
///
/// The simplest way to create this type is to create a [`Duration`] and covert it into it.
/// ```
/// use std::time::Duration;
/// use mpris::MprisDuration;
///
/// let m_dur = MprisDuration::new_from_duration(Duration::from_secs(10));
/// ```
///
/// All of the [`TryFrom`] implementations for [`MprisDuration`] will fail if the value can't be
/// losslessly converted to a valid [`MprisDuration`]. In case you just want to create a valid
/// [`MprisDuration`] and don't are about about being lossless you can use the `new_from_*` methods.
/// ```
/// use std::time::Duration;
/// use mpris::MprisDuration;
///
/// let m_dur = MprisDuration::new_from_i64(-42_i64);
/// assert_eq!(m_dur.as_u64(), 0);
///
/// let dur_big = MprisDuration::new_from_duration(Duration::from_secs(u64::MAX));
/// assert_eq!(dur_big.as_i64(), i64::MAX)
/// ```
/// ## Ops
/// [`MprisDuration`] implements [`Add`], [`Sub`], [`Mul`] and [`Div`] for itself, [`Duration`] and
/// [`u64`]. The values will stay valid.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "i64", into = "i64")
)]
pub struct MprisDuration(u64);

impl MprisDuration {
    /// Lossily creates a new valid [`MprisDuration`] from a [`u64`].
    ///
    /// If the value is too big it will be reduced to [`i64::MAX`]. If you want a lossless
    /// conversion that can fail you should use [`TryFrom<u64>`].
    pub fn new_from_u64(micros: u64) -> Self {
        Self(micros.clamp(0, MAX))
    }

    /// Lossily creates a new valid [`MprisDuration`] from a [`i64`].
    ///
    /// Negative values will be changed to `0`. If you want a lossless conversion that can fail you
    /// should use [`TryFrom<i64>`].
    pub fn new_from_i64(micros: i64) -> Self {
        Self(micros.clamp(0, i64::MAX) as u64)
    }

    /// Lossily creates a new valid [`MprisDuration`] from a [`Duration`].
    ///
    /// If [`Duration`]'s values as microseconds is too big it will be reduced to [`i64::MAX`]. If
    /// you want a lossless conversion that can fail you should use [`TryFrom<Duration>`].
    pub fn new_from_duration(duration: Duration) -> Self {
        Self(duration.as_micros().clamp(0, MAX as u128) as u64)
    }

    /// Creates a new [`MprisDuration`] with the biggest possible value
    pub fn new_max() -> Self {
        Self(MAX)
    }

    /// Returns a [`u64`] equal to the [`MprisDuration`]'s value as microseconds
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Returns a [`i64`] equal to the [`MprisDuration`]'s value as microseconds
    pub fn as_i64(&self) -> i64 {
        self.0 as i64
    }
}

impl From<MprisDuration> for Duration {
    fn from(value: MprisDuration) -> Self {
        Duration::from_micros(value.0)
    }
}

impl TryFrom<Duration> for MprisDuration {
    type Error = InvalidMprisDuration;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        u64::try_from(value.as_micros())
            .or(Err(InvalidMprisDuration::new_too_big()))?
            .try_into()
    }
}

impl From<MprisDuration> for i64 {
    fn from(value: MprisDuration) -> Self {
        value.0 as i64
    }
}

impl From<MprisDuration> for u64 {
    fn from(value: MprisDuration) -> Self {
        value.0
    }
}

impl TryFrom<i64> for MprisDuration {
    type Error = InvalidMprisDuration;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value < 0 {
            Err(InvalidMprisDuration::new_negative())
        } else {
            Ok(Self(value as u64))
        }
    }
}

impl TryFrom<u64> for MprisDuration {
    type Error = InvalidMprisDuration;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > MAX {
            Err(InvalidMprisDuration::new_too_big())
        } else {
            Ok(Self(value))
        }
    }
}

impl TryFrom<MetadataValue> for MprisDuration {
    type Error = InvalidMprisDuration;

    fn try_from(value: MetadataValue) -> Result<Self, Self::Error> {
        match value {
            MetadataValue::SignedInt(int) => int.try_into(),
            MetadataValue::UnsignedInt(int) => int.try_into(),
            _ => Err(InvalidMprisDuration::from("unsupported MetadataValue type")),
        }
    }
}

macro_rules! impl_math {
    ($trait:ident, $method:ident, $sat:ident) => {
        impl $trait<u64> for MprisDuration {
            type Output = Self;

            fn $method(self, rhs: u64) -> Self::Output {
                Self(self.0.$sat(rhs).clamp(0, MAX))
            }
        }

        impl $trait<&u64> for MprisDuration {
            type Output = Self;

            fn $method(self, rhs: &u64) -> Self::Output {
                Self::$method(self, *rhs)
            }
        }

        impl $trait<MprisDuration> for MprisDuration {
            type Output = Self;

            fn $method(self, rhs: MprisDuration) -> Self::Output {
                Self::$method(self, rhs.as_u64())
            }
        }

        impl $trait<&MprisDuration> for MprisDuration {
            type Output = Self;

            fn $method(self, rhs: &MprisDuration) -> Self::Output {
                Self::$method(self, rhs.as_u64())
            }
        }

        impl $trait<Duration> for MprisDuration {
            type Output = Self;

            fn $method(self, rhs: Duration) -> Self::Output {
                Self::$method(self, rhs.as_micros().clamp(0, MAX as u128) as u64)
            }
        }

        impl $trait<&Duration> for MprisDuration {
            type Output = Self;

            fn $method(self, rhs: &Duration) -> Self::Output {
                Self::$method(self, rhs.as_micros().clamp(0, MAX as u128) as u64)
            }
        }
    };
}

impl_math!(Mul, mul, saturating_mul);
// Using regular div because of the current MSRV
// Can you even underflow a u64 with div?
impl_math!(Div, div, div);
impl_math!(Add, add, saturating_add);
impl_math!(Sub, sub, saturating_sub);

#[cfg(test)]
mod mrpis_duration_tests {
    use super::*;

    #[test]
    fn new() {
        assert_eq!(MprisDuration::new_max(), MprisDuration(MAX));
        assert_eq!(MprisDuration::new_from_u64(0), MprisDuration(0));
        assert_eq!(MprisDuration::default(), MprisDuration(0));
        assert_eq!(
            MprisDuration::new_from_u64(u64::MAX),
            MprisDuration::new_max()
        );
    }

    #[test]
    fn into_duration() {
        assert_eq!(
            Duration::from(MprisDuration::new_from_u64(0)),
            Duration::from_micros(0)
        );
        assert_eq!(
            Duration::from(MprisDuration::new_from_u64(123456789)),
            Duration::from_micros(123456789)
        );
        assert_eq!(
            Duration::from(MprisDuration::new_max()),
            Duration::from_micros(MAX)
        );
    }

    #[test]
    fn try_from_duration() {
        assert_eq!(
            MprisDuration::try_from(Duration::default()),
            Ok(MprisDuration::default())
        );
        assert_eq!(
            MprisDuration::try_from(Duration::from_micros(MAX)),
            Ok(MprisDuration::new_max())
        );

        assert!(MprisDuration::try_from(Duration::from_micros(MAX + 1)).is_err());
    }

    #[test]
    fn into_ints() {
        let d = MprisDuration::default();
        assert_eq!(i64::from(d), 0);
        assert_eq!(u64::from(d), 0);
        let d_max = MprisDuration::new_max();
        assert_eq!(i64::from(d_max), i64::MAX);
        assert_eq!(u64::from(d_max), MAX);
    }

    #[test]
    fn try_from_ints() {
        assert!(MprisDuration::try_from(i64::MIN).is_err());
        assert_eq!(MprisDuration::try_from(0_i64), Ok(MprisDuration::default()));
        assert_eq!(
            MprisDuration::try_from(i64::MAX),
            Ok(MprisDuration::new_max())
        );
        assert_eq!(MprisDuration::try_from(0_u64), Ok(MprisDuration::default()));
        assert!(MprisDuration::try_from(MAX + 1).is_err());
    }

    #[test]
    fn try_from_metadata_value() {
        assert!(MprisDuration::try_from(MetadataValue::Boolean(false)).is_err());
        assert!(MprisDuration::try_from(MetadataValue::Float(0.0)).is_err());
        assert!(MprisDuration::try_from(MetadataValue::SignedInt(0)).is_ok());
        assert!(MprisDuration::try_from(MetadataValue::UnsignedInt(0)).is_ok());
        assert!(MprisDuration::try_from(MetadataValue::String(String::new())).is_err());
        assert!(MprisDuration::try_from(MetadataValue::Strings(vec![])).is_err());
        assert!(MprisDuration::try_from(MetadataValue::Unsupported).is_err());
    }
}

#[cfg(test)]
mod ops_tests {
    use super::*;

    macro_rules! gen_ops_tests {
        ($type:expr) => {
            assert_eq!(
                MprisDuration::new_from_u64(1) * $type(10_u64),
                MprisDuration::new_from_u64(10)
            );
            #[allow(clippy::erasing_op)]
            {
                assert_eq!(
                    MprisDuration::new_from_u64(1) * $type(0_u64),
                    MprisDuration::new_from_u64(0)
                );
            }
            assert_eq!(
                MprisDuration::new_max() * $type(2_u64),
                MprisDuration::new_max()
            );

            assert_eq!(
                MprisDuration::new_from_u64(0) / $type(1_u64),
                MprisDuration::new_from_u64(0)
            );
            assert_eq!(
                MprisDuration::new_from_u64(10) / $type(3_u64),
                MprisDuration::new_from_u64(10 / 3)
            );
            assert_eq!(
                MprisDuration::new_max() / $type(MAX),
                MprisDuration::new_from_u64(1)
            );
            assert_eq!(
                MprisDuration::new_from_u64(1) / $type(MAX),
                MprisDuration::new_from_u64(0)
            );

            assert_eq!(
                MprisDuration::new_from_u64(0) + $type(1_u64),
                MprisDuration::new_from_u64(1)
            );
            assert_eq!(
                MprisDuration::new_max() + $type(1_u64),
                MprisDuration::new_max()
            );

            assert_eq!(
                MprisDuration::new_from_u64(0) - $type(1_u64),
                MprisDuration::new_from_u64(0)
            );
            assert_eq!(
                MprisDuration::new_from_u64(10) - $type(1_u64),
                MprisDuration::new_from_u64(9)
            );
        };
    }

    #[test]
    fn u64() {
        gen_ops_tests!(u64::from);
    }

    #[test]
    fn mpris_duration() {
        gen_ops_tests!(MprisDuration::new_from_u64);
    }

    #[test]
    fn duration() {
        gen_ops_tests!(Duration::from_micros);
    }
}

#[cfg(all(test, feature = "serde"))]
mod mpris_duration_serde_tests {
    use super::*;
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};

    const MIN_TOKENS: [Token; 1] = get_tokens(0);
    const MAX_TOKENS: [Token; 1] = get_tokens(MAX as i64);

    const fn get_tokens(x: i64) -> [Token; 1] {
        [Token::I64(x)]
    }

    #[test]
    fn ser_and_deser() {
        assert_tokens(&MprisDuration::default(), &MIN_TOKENS);
        assert_tokens(&MprisDuration::new_max(), &MAX_TOKENS);
    }

    #[test]
    fn invalid_deser() {
        assert_de_tokens_error::<MprisDuration>(
            &get_tokens(-1),
            &InvalidMprisDuration::new_negative().0,
        );
    }
}
