use error::Result;
use handle::{Handle, ID, FromID, get_id_type};
use object::Object;

use ffi::h5i::{H5I_DATATYPE, hid_t};
use ffi::h5t::{H5T_INTEGER, H5T_FLOAT, H5T_NO_CLASS, H5T_NCLASSES, H5T_ORDER_BE, H5T_ORDER_LE,
               H5T_SGN_2, H5Tcopy, H5Tget_class, H5Tget_order, H5Tget_offset, H5Tget_sign,
               H5Tget_precision};

#[cfg(target_endian = "big")]
use globals::{
    H5T_STD_I8BE, H5T_STD_I16BE,
    H5T_STD_I32BE, H5T_STD_I64BE,
    H5T_STD_U8BE, H5T_STD_U16BE,
    H5T_STD_U32BE, H5T_STD_U64BE,
    H5T_IEEE_F32BE, H5T_IEEE_F64BE,
};

#[cfg(target_endian = "little")]
use globals::{
    H5T_STD_I8LE, H5T_STD_I16LE,
    H5T_STD_I32LE, H5T_STD_I64LE,
    H5T_STD_U8LE, H5T_STD_U16LE,
    H5T_STD_U32LE, H5T_STD_U64LE,
    H5T_IEEE_F32LE, H5T_IEEE_F64LE,
};

pub enum Datatype {
    Integer(IntegerDatatype),
    Float(FloatDatatype),
}

macro_rules! def_atomic {
    ($name:ident, $h5t:ident) => (
        pub struct $name {
            handle: Handle,
        }

        impl ID for $name {
            fn id(&self) -> hid_t {
                self.handle.id()
            }
        }

        impl FromID for $name {
            fn from_id(id: hid_t) -> Result<$name> {
                h5lock!({
                    if get_id_type(id) != H5I_DATATYPE {
                        return Err(From::from(format!("Invalid datatype id: {}", id)));
                    }
                    let cls = H5Tget_class(id);
                    if cls != $h5t {
                        return Err(From::from(format!("Invalid datatype class: {:?}", cls)));
                    }
                    Ok($name { handle: try!(Handle::new(id)) })
                })
            }
        }

        impl Object for $name {}
        impl AtomicDatatype for $name{}
    )
}

def_atomic!(IntegerDatatype, H5T_INTEGER);

impl IntegerDatatype {
    /// Returns true if the datatype is signed.
    pub fn is_signed(&self) -> bool {
        h5lock!(H5Tget_sign(self.id()) == H5T_SGN_2)
    }
}

def_atomic!(FloatDatatype, H5T_FLOAT);

pub trait AtomicDatatype: ID {
    /// Returns true if the datatype byte order is big endian.
    fn is_be(&self) -> bool {
        h5lock!(H5Tget_order(self.id()) == H5T_ORDER_BE)

    }

    /// Returns true if the datatype byte order is little endian.
    fn is_le(&self) -> bool {
        h5lock!(H5Tget_order(self.id()) == H5T_ORDER_LE)
    }

    /// Get the offset of the first significant bit.
    fn offset(&self) -> usize {
        h5call!(H5Tget_offset(self.id())).unwrap_or(0) as usize
    }

    /// Get the number of significant bits, excluding padding.
    fn precision(&self) -> usize {
        h5call!(H5Tget_precision(self.id())).unwrap_or(0) as usize
    }
}

pub trait ToDatatype {
    fn to_datatype() -> Result<Datatype>;
}

macro_rules! impl_atomic {
    ($tp:ty, $be:ident, $le:ident) => (
        impl ToDatatype for $tp {
            #[cfg(target_endian = "big")]
            fn to_datatype() -> Result<Datatype> {
                Datatype::from_id(h5try!(H5Tcopy(*$be)))
            }

            #[cfg(target_endian = "little")]
            fn to_datatype() -> Result<Datatype> {
                Datatype::from_id(h5try!(H5Tcopy(*$le)))
            }
        }
    )
}

impl_atomic!(bool, H5T_STD_U8BE, H5T_STD_U8LE);

impl_atomic!(i8, H5T_STD_I8BE, H5T_STD_I8LE);
impl_atomic!(i16, H5T_STD_I16BE, H5T_STD_I16LE);
impl_atomic!(i32, H5T_STD_I32BE, H5T_STD_I32LE);
impl_atomic!(i64, H5T_STD_I64BE, H5T_STD_I64LE);

impl_atomic!(u8, H5T_STD_U8BE, H5T_STD_U8LE);
impl_atomic!(u16, H5T_STD_U16BE, H5T_STD_U16LE);
impl_atomic!(u32, H5T_STD_U32BE, H5T_STD_U32LE);
impl_atomic!(u64, H5T_STD_U64BE, H5T_STD_U64LE);

impl_atomic!(f32, H5T_IEEE_F32BE, H5T_IEEE_F32LE);
impl_atomic!(f64, H5T_IEEE_F64BE, H5T_IEEE_F64LE);

#[cfg(target_pointer_width = "32")] impl_atomic!(usize, H5T_STD_U32BE, H5T_STD_U32LE);
#[cfg(target_pointer_width = "32")] impl_atomic!(isize, H5T_STD_I32BE, H5T_STD_I32LE);

#[cfg(target_pointer_width = "64")] impl_atomic!(usize, H5T_STD_U64BE, H5T_STD_U64LE);
#[cfg(target_pointer_width = "64")] impl_atomic!(isize, H5T_STD_I64BE, H5T_STD_I64LE);

impl ID for Datatype {
    fn id(&self) -> hid_t {
        match *self {
            Datatype::Integer(ref dt) => dt.id(),
            Datatype::Float(ref dt)   => dt.id(),
        }
    }
}

impl FromID for Datatype {
    fn from_id(id: hid_t) -> Result<Datatype> {
        h5lock!({
            match get_id_type(id) {
                H5I_DATATYPE => {
                    match H5Tget_class(id) {
                        H5T_INTEGER  => Ok(Datatype::Integer(try!(IntegerDatatype::from_id(id)))),
                        H5T_FLOAT    => Ok(Datatype::Float(try!(FloatDatatype::from_id(id)))),
                        H5T_NO_CLASS |
                        H5T_NCLASSES => Err(From::from("Invalid datatype class")),
                        cls          => Err(From::from(format!("Unsupported datatype: {:?}", cls)))
                    }
                },
                _ => Err(From::from(format!("Invalid datatype id: {}", id))),
            }
        })
    }
}

impl Object for Datatype {}

#[cfg(test)]
mod tests {
    use super::{Datatype, AtomicDatatype, ToDatatype};
    use super::Datatype::*;
    use handle::FromID;
    use ffi::h5i::H5I_INVALID_HID;
    use ffi::h5t::H5Tcopy;
    use globals::H5T_STD_REF_OBJ;

    #[cfg(target_endian = "big")] const IS_BE: bool = true;
    #[cfg(target_endian = "big")] const IS_LE: bool = false;

    #[cfg(target_endian = "little")] const IS_BE: bool = false;
    #[cfg(target_endian = "little")] const IS_LE: bool = true;

    #[cfg(target_pointer_width = "32")] const POINTER_WIDTH: usize = 32;
    #[cfg(target_pointer_width = "64")] const POINTER_WIDTH: usize = 64;

    #[test]
    pub fn test_invalid_datatype() {
        assert_err!(Datatype::from_id(H5I_INVALID_HID), "Invalid datatype id");
        assert_err!(Datatype::from_id(h5lock!(H5Tcopy(*H5T_STD_REF_OBJ))), "Unsupported datatype");
    }

    #[test]
    pub fn test_atomic_datatype() {
        macro_rules! test_integer {
            ($tp:ty, $signed:expr, $precision:expr) => (
                match <$tp as ToDatatype>::to_datatype().unwrap() {
                    Datatype::Integer(dt) => {
                        assert_eq!(dt.is_be(), IS_BE);
                        assert_eq!(dt.is_le(), IS_LE);
                        assert_eq!(dt.offset(), 0);
                        assert_eq!(dt.precision(), $precision);
                        assert_eq!(dt.is_signed(), $signed);
                    },
                    _ => panic!("Integer datatype expected")
                }
            )
        }

        macro_rules! test_float {
            ($tp:ty, $precision:expr) => (
                match <$tp as ToDatatype>::to_datatype().unwrap() {
                    Datatype::Float(dt) => {
                        assert_eq!(dt.is_be(), IS_BE);
                        assert_eq!(dt.is_le(), IS_LE);
                        assert_eq!(dt.offset(), 0);
                        assert_eq!(dt.precision(), $precision)
                    },
                    _ => panic!("Float datatype expected")
                }
            )
        }

        test_integer!(bool, false, 8);

        test_integer!(i8, true, 8);
        test_integer!(i16, true, 16);
        test_integer!(i32, true, 32);
        test_integer!(i64, true, 64);

        test_integer!(u8, false, 8);
        test_integer!(u16, false, 16);
        test_integer!(u32, false, 32);
        test_integer!(u64, false, 64);

        test_float!(f32, 32);
        test_float!(f64, 64);

        test_integer!(isize, true, POINTER_WIDTH);
        test_integer!(usize, false, POINTER_WIDTH);
    }
}
