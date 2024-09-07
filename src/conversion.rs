use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::hash::{BuildHasher, Hash};
use std::os::raw::c_int;
use std::string::String as StdString;
use std::{slice, str};

use bstr::{BStr, BString};
use num_traits::cast;

use crate::error::{Error, Result};
use crate::function::Function;
use crate::state::{Lua, RawLua};
use crate::string::String;
use crate::table::Table;
use crate::thread::Thread;
use crate::types::{LightUserData, MaybeSend, RegistryKey};
use crate::userdata::{AnyUserData, UserData};
use crate::value::{FromLua, IntoLua, Nil, Value};

impl IntoLua for Value {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(self)
    }
}

impl IntoLua for &Value {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(self.clone())
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        lua.push_value(self)
    }
}

impl FromLua for Value {
    #[inline]
    fn from_lua(lua_value: Value, _: &Lua) -> Result<Self> {
        Ok(lua_value)
    }
}

impl IntoLua for String {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::String(self))
    }
}

impl IntoLua for &String {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::String(self.clone()))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        lua.push_ref(&self.0);
        Ok(())
    }
}

impl FromLua for String {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<String> {
        let ty = value.type_name();
        lua.coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "string",
                message: Some("expected string or number".to_string()),
            })
    }

    unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
        let state = lua.state();
        let type_id = ffi::lua_type(state, idx);
        if type_id == ffi::LUA_TSTRING {
            ffi::lua_xpush(state, lua.ref_thread(), idx);
            return Ok(String(lua.pop_ref_thread()));
        }
        // Fallback to default
        Self::from_lua(lua.stack_value(idx, Some(type_id)), lua.lua())
    }
}

impl IntoLua for Table {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Table(self))
    }
}

impl IntoLua for &Table {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Table(self.clone()))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        lua.push_ref(&self.0);
        Ok(())
    }
}

impl FromLua for Table {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Table> {
        match value {
            Value::Table(table) => Ok(table),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "table",
                message: None,
            }),
        }
    }
}

impl IntoLua for Function {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Function(self))
    }
}

impl IntoLua for &Function {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Function(self.clone()))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        lua.push_ref(&self.0);
        Ok(())
    }
}

impl FromLua for Function {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Function> {
        match value {
            Value::Function(table) => Ok(table),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "function",
                message: None,
            }),
        }
    }
}

impl IntoLua for Thread {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Thread(self))
    }
}

impl IntoLua for &Thread {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Thread(self.clone()))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        lua.push_ref(&self.0);
        Ok(())
    }
}

impl FromLua for Thread {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Thread> {
        match value {
            Value::Thread(t) => Ok(t),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "thread",
                message: None,
            }),
        }
    }
}

impl IntoLua for AnyUserData {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::UserData(self))
    }
}

impl IntoLua for &AnyUserData {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::UserData(self.clone()))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        lua.push_ref(&self.0);
        Ok(())
    }
}

impl FromLua for AnyUserData {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<AnyUserData> {
        match value {
            Value::UserData(ud) => Ok(ud),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "userdata",
                message: None,
            }),
        }
    }
}

impl<T: UserData + MaybeSend + 'static> IntoLua for T {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(lua.create_userdata(self)?))
    }
}

impl IntoLua for Error {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Error(Box::new(self)))
    }
}

impl FromLua for Error {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Error> {
        match value {
            Value::Error(err) => Ok(*err),
            val => Ok(Error::runtime(
                lua.coerce_string(val)?
                    .and_then(|s| Some(s.to_str().ok()?.to_owned()))
                    .unwrap_or_else(|| "<unprintable error>".to_owned()),
            )),
        }
    }
}

impl IntoLua for RegistryKey {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        lua.registry_value(&self)
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        <&RegistryKey>::push_into_stack(&self, lua)
    }
}

impl IntoLua for &RegistryKey {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        lua.registry_value(self)
    }

    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        if !lua.owns_registry_value(self) {
            return Err(Error::MismatchedRegistryKey);
        }

        match self.id() {
            ffi::LUA_REFNIL => ffi::lua_pushnil(lua.state()),
            id => {
                ffi::lua_rawgeti(lua.state(), ffi::LUA_REGISTRYINDEX, id as _);
            }
        }
        Ok(())
    }
}

impl FromLua for RegistryKey {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<RegistryKey> {
        lua.create_registry_value(value)
    }
}

impl IntoLua for bool {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Boolean(self))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        ffi::lua_pushboolean(lua.state(), self as c_int);
        Ok(())
    }
}

impl FromLua for bool {
    #[inline]
    fn from_lua(v: Value, _: &Lua) -> Result<Self> {
        match v {
            Value::Nil => Ok(false),
            Value::Boolean(b) => Ok(b),
            _ => Ok(true),
        }
    }

    #[inline]
    unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
        Ok(ffi::lua_toboolean(lua.state(), idx) != 0)
    }
}

impl IntoLua for LightUserData {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::LightUserData(self))
    }
}

impl FromLua for LightUserData {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::LightUserData(ud) => Ok(ud),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "light userdata",
                message: None,
            }),
        }
    }
}

#[cfg(feature = "luau")]
impl IntoLua for crate::types::Vector {
    #[inline]
    fn into_lua(self, _: &Lua) -> Result<Value> {
        Ok(Value::Vector(self))
    }
}

#[cfg(feature = "luau")]
impl FromLua for crate::types::Vector {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::Vector(v) => Ok(v),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "vector",
                message: None,
            }),
        }
    }
}

impl IntoLua for StdString {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)?))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        push_bytes_into_stack(self, lua)
    }
}

impl FromLua for StdString {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        Ok(lua
            .coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "String",
                message: Some("expected string or number".to_string()),
            })?
            .to_str()?
            .to_owned())
    }

    #[inline]
    unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
        let state = lua.state();
        let type_id = ffi::lua_type(state, idx);
        if type_id == ffi::LUA_TSTRING {
            let mut size = 0;
            let data = ffi::lua_tolstring(state, idx, &mut size);
            let bytes = slice::from_raw_parts(data as *const u8, size);
            return str::from_utf8(bytes)
                .map(|s| s.to_owned())
                .map_err(|e| Error::FromLuaConversionError {
                    from: "string",
                    to: "String",
                    message: Some(e.to_string()),
                });
        }
        // Fallback to default
        Self::from_lua(lua.stack_value(idx, Some(type_id)), lua.lua())
    }
}

impl IntoLua for &str {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)?))
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        push_bytes_into_stack(self, lua)
    }
}

impl IntoLua for Cow<'_, str> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.as_bytes())?))
    }
}

impl IntoLua for Box<str> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(&*self)?))
    }
}

impl FromLua for Box<str> {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        Ok(lua
            .coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "Box<str>",
                message: Some("expected string or number".to_string()),
            })?
            .to_str()?
            .to_owned()
            .into_boxed_str())
    }
}

impl IntoLua for CString {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.as_bytes())?))
    }
}

impl FromLua for CString {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        let string = lua
            .coerce_string(value)?
            .ok_or_else(|| Error::FromLuaConversionError {
                from: ty,
                to: "CString",
                message: Some("expected string or number".to_string()),
            })?;

        match CStr::from_bytes_with_nul(&string.as_bytes_with_nul()) {
            Ok(s) => Ok(s.into()),
            Err(_) => Err(Error::FromLuaConversionError {
                from: ty,
                to: "CString",
                message: Some("invalid C-style string".to_string()),
            }),
        }
    }
}

impl IntoLua for &CStr {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.to_bytes())?))
    }
}

impl IntoLua for Cow<'_, CStr> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.to_bytes())?))
    }
}

impl IntoLua for BString {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)?))
    }
}

impl FromLua for BString {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let ty = value.type_name();
        match value {
            Value::String(s) => Ok((*s.as_bytes()).into()),
            #[cfg(feature = "luau")]
            Value::UserData(ud) if ud.1 == crate::types::SubtypeId::Buffer => unsafe {
                let lua = ud.0.lua.lock();
                let mut size = 0usize;
                let buf = ffi::lua_tobuffer(lua.ref_thread(), ud.0.index, &mut size);
                mlua_assert!(!buf.is_null(), "invalid Luau buffer");
                Ok(slice::from_raw_parts(buf as *const u8, size).into())
            },
            _ => Ok((*lua
                .coerce_string(value)?
                .ok_or_else(|| Error::FromLuaConversionError {
                    from: ty,
                    to: "BString",
                    message: Some("expected string or number".to_string()),
                })?
                .as_bytes())
            .into()),
        }
    }

    unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
        let state = lua.state();
        match ffi::lua_type(state, idx) {
            ffi::LUA_TSTRING => {
                let mut size = 0;
                let data = ffi::lua_tolstring(state, idx, &mut size);
                Ok(slice::from_raw_parts(data as *const u8, size).into())
            }
            #[cfg(feature = "luau")]
            ffi::LUA_TBUFFER => {
                let mut size = 0;
                let buf = ffi::lua_tobuffer(state, idx, &mut size);
                mlua_assert!(!buf.is_null(), "invalid Luau buffer");
                Ok(slice::from_raw_parts(buf as *const u8, size).into())
            }
            type_id => {
                // Fallback to default
                Self::from_lua(lua.stack_value(idx, Some(type_id)), lua.lua())
            }
        }
    }
}

impl IntoLua for &BStr {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)?))
    }
}

#[inline]
unsafe fn push_bytes_into_stack<T>(this: T, lua: &RawLua) -> Result<()>
where
    T: IntoLua + AsRef<[u8]>,
{
    let bytes = this.as_ref();
    if lua.unlikely_memory_error() && bytes.len() < (1 << 30) {
        // Fast path: push directly into the Lua stack.
        ffi::lua_pushlstring(lua.state(), bytes.as_ptr() as *const _, bytes.len());
        return Ok(());
    }
    // Fallback to default
    lua.push_value(&T::into_lua(this, lua.lua())?)
}

macro_rules! lua_convert_int {
    ($x:ty) => {
        impl IntoLua for $x {
            #[inline]
            fn into_lua(self, _: &Lua) -> Result<Value> {
                cast(self)
                    .map(Value::Integer)
                    .or_else(|| cast(self).map(Value::Number))
                    // This is impossible error because conversion to Number never fails
                    .ok_or_else(|| Error::ToLuaConversionError {
                        from: stringify!($x),
                        to: "number",
                        message: Some("out of range".to_owned()),
                    })
            }

            #[inline]
            unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
                match cast(self) {
                    Some(i) => ffi::lua_pushinteger(lua.state(), i),
                    None => ffi::lua_pushnumber(lua.state(), self as ffi::lua_Number),
                }
                Ok(())
            }
        }

        impl FromLua for $x {
            #[inline]
            fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
                let ty = value.type_name();
                (match value {
                    Value::Integer(i) => cast(i),
                    Value::Number(n) => cast(n),
                    _ => {
                        if let Some(i) = lua.coerce_integer(value.clone())? {
                            cast(i)
                        } else {
                            cast(
                                lua.coerce_number(value)?
                                    .ok_or_else(|| Error::FromLuaConversionError {
                                        from: ty,
                                        to: stringify!($x),
                                        message: Some(
                                            "expected number or string coercible to number".to_string(),
                                        ),
                                    })?,
                            )
                        }
                    }
                })
                .ok_or_else(|| Error::FromLuaConversionError {
                    from: ty,
                    to: stringify!($x),
                    message: Some("out of range".to_owned()),
                })
            }

            unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
                let state = lua.state();
                let type_id = ffi::lua_type(state, idx);
                if type_id == ffi::LUA_TNUMBER {
                    let mut ok = 0;
                    let i = ffi::lua_tointegerx(state, idx, &mut ok);
                    if ok != 0 {
                        return cast(i).ok_or_else(|| Error::FromLuaConversionError {
                            from: "integer",
                            to: stringify!($x),
                            message: Some("out of range".to_owned()),
                        });
                    }
                }
                // Fallback to default
                Self::from_lua(lua.stack_value(idx, Some(type_id)), lua.lua())
            }
        }
    };
}

lua_convert_int!(i8);
lua_convert_int!(u8);
lua_convert_int!(i16);
lua_convert_int!(u16);
lua_convert_int!(i32);
lua_convert_int!(u32);
lua_convert_int!(i64);
lua_convert_int!(u64);
lua_convert_int!(i128);
lua_convert_int!(u128);
lua_convert_int!(isize);
lua_convert_int!(usize);

macro_rules! lua_convert_float {
    ($x:ty) => {
        impl IntoLua for $x {
            #[inline]
            fn into_lua(self, _: &Lua) -> Result<Value> {
                cast(self)
                    .ok_or_else(|| Error::ToLuaConversionError {
                        from: stringify!($x),
                        to: "number",
                        message: Some("out of range".to_string()),
                    })
                    .map(Value::Number)
            }
        }

        impl FromLua for $x {
            #[inline]
            fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
                let ty = value.type_name();
                lua.coerce_number(value)?
                    .ok_or_else(|| Error::FromLuaConversionError {
                        from: ty,
                        to: stringify!($x),
                        message: Some("expected number or string coercible to number".to_string()),
                    })
                    .and_then(|n| {
                        cast(n).ok_or_else(|| Error::FromLuaConversionError {
                            from: ty,
                            to: stringify!($x),
                            message: Some("number out of range".to_string()),
                        })
                    })
            }

            unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
                let state = lua.state();
                let type_id = ffi::lua_type(state, idx);
                if type_id == ffi::LUA_TNUMBER {
                    let mut ok = 0;
                    let i = ffi::lua_tonumberx(state, idx, &mut ok);
                    if ok != 0 {
                        return cast(i).ok_or_else(|| Error::FromLuaConversionError {
                            from: "number",
                            to: stringify!($x),
                            message: Some("out of range".to_owned()),
                        });
                    }
                }
                // Fallback to default
                Self::from_lua(lua.stack_value(idx, Some(type_id)), lua.lua())
            }
        }
    };
}

lua_convert_float!(f32);
lua_convert_float!(f64);

impl<T> IntoLua for &[T]
where
    T: IntoLua + Clone,
{
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self.iter().cloned())?))
    }
}

impl<T, const N: usize> IntoLua for [T; N]
where
    T: IntoLua,
{
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self)?))
    }
}

impl<T, const N: usize> FromLua for [T; N]
where
    T: FromLua,
{
    #[inline]
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            #[cfg(feature = "luau")]
            #[rustfmt::skip]
            Value::Vector(v) if N == crate::types::Vector::SIZE => unsafe {
                use std::{mem, ptr};
                let mut arr: [mem::MaybeUninit<T>; N] = mem::MaybeUninit::uninit().assume_init();
                ptr::write(arr[0].as_mut_ptr() , T::from_lua(Value::Number(v.x() as _), _lua)?);
                ptr::write(arr[1].as_mut_ptr(), T::from_lua(Value::Number(v.y() as _), _lua)?);
                ptr::write(arr[2].as_mut_ptr(), T::from_lua(Value::Number(v.z() as _), _lua)?);
                #[cfg(feature = "luau-vector4")]
                ptr::write(arr[3].as_mut_ptr(), T::from_lua(Value::Number(v.w() as _), _lua)?);
                Ok(mem::transmute_copy(&arr))
            },
            Value::Table(table) => {
                let vec = table.sequence_values().collect::<Result<Vec<_>>>()?;
                vec.try_into()
                    .map_err(|vec: Vec<T>| Error::FromLuaConversionError {
                        from: "table",
                        to: "Array",
                        message: Some(format!("expected table of length {}, got {}", N, vec.len())),
                    })
            }
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Array",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<T: IntoLua> IntoLua for Box<[T]> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self.into_vec())?))
    }
}

impl<T: FromLua> FromLua for Box<[T]> {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Vec::<T>::from_lua(value, lua)?.into_boxed_slice())
    }
}

impl<T: IntoLua> IntoLua for Vec<T> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_sequence_from(self)?))
    }
}

impl<T: FromLua> FromLua for Vec<T> {
    #[inline]
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Table(table) => table.sequence_values().collect(),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Vec",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<K: Eq + Hash + IntoLua, V: IntoLua, S: BuildHasher> IntoLua for HashMap<K, V, S> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_table_from(self)?))
    }
}

impl<K: Eq + Hash + FromLua, V: FromLua, S: BuildHasher + Default> FromLua for HashMap<K, V, S> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        if let Value::Table(table) = value {
            table.pairs().collect()
        } else {
            Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HashMap",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<K: Ord + IntoLua, V: IntoLua> IntoLua for BTreeMap<K, V> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(lua.create_table_from(self)?))
    }
}

impl<K: Ord + FromLua, V: FromLua> FromLua for BTreeMap<K, V> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        if let Value::Table(table) = value {
            table.pairs().collect()
        } else {
            Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "BTreeMap",
                message: Some("expected table".to_string()),
            })
        }
    }
}

impl<T: Eq + Hash + IntoLua, S: BuildHasher> IntoLua for HashSet<T, S> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(
            lua.create_table_from(self.into_iter().map(|val| (val, true)))?,
        ))
    }
}

impl<T: Eq + Hash + FromLua, S: BuildHasher + Default> FromLua for HashSet<T, S> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::Table(table) if table.raw_len() > 0 => table.sequence_values().collect(),
            Value::Table(table) => table.pairs::<T, Value>().map(|res| res.map(|(k, _)| k)).collect(),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "HashSet",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<T: Ord + IntoLua> IntoLua for BTreeSet<T> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::Table(
            lua.create_table_from(self.into_iter().map(|val| (val, true)))?,
        ))
    }
}

impl<T: Ord + FromLua> FromLua for BTreeSet<T> {
    #[inline]
    fn from_lua(value: Value, _: &Lua) -> Result<Self> {
        match value {
            Value::Table(table) if table.raw_len() > 0 => table.sequence_values().collect(),
            Value::Table(table) => table.pairs::<T, Value>().map(|res| res.map(|(k, _)| k)).collect(),
            _ => Err(Error::FromLuaConversionError {
                from: value.type_name(),
                to: "BTreeSet",
                message: Some("expected table".to_string()),
            }),
        }
    }
}

impl<T: IntoLua> IntoLua for Option<T> {
    #[inline]
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        match self {
            Some(val) => val.into_lua(lua),
            None => Ok(Nil),
        }
    }

    #[inline]
    unsafe fn push_into_stack(self, lua: &RawLua) -> Result<()> {
        match self {
            Some(val) => val.push_into_stack(lua)?,
            None => ffi::lua_pushnil(lua.state()),
        }
        Ok(())
    }
}

impl<T: FromLua> FromLua for Option<T> {
    #[inline]
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        match value {
            Nil => Ok(None),
            value => Ok(Some(T::from_lua(value, lua)?)),
        }
    }

    #[inline]
    unsafe fn from_stack(idx: c_int, lua: &RawLua) -> Result<Self> {
        match ffi::lua_type(lua.state(), idx) {
            ffi::LUA_TNIL => Ok(None),
            _ => Ok(Some(T::from_stack(idx, lua)?)),
        }
    }
}
