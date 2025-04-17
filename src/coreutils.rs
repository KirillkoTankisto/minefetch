/*
Yep, it's uutils code. It's under MIT License which you can read here
https://github.com/uutils/coreutils/blob/main/LICENSE
Copyright (c) uutils developers

btw, thanks for this code!
*/

use libc::{c_char, getpwnam, getpwuid, passwd, uid_t};

use std::ffi::OsString;
use std::ffi::{CStr, CString};
use std::io;
use std::io::Error as IOError;
use std::io::ErrorKind;
use std::io::Result as IOResult;
use std::marker::Sized;
use std::ptr;
use std::sync::{LazyLock, Mutex};

fn cstr2string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() })
    }
}

#[derive(Clone, Debug)]
pub struct Passwd {
    pub name: String,
}

impl Passwd {
    unsafe fn from_raw(raw: passwd) -> Self {
        Self {
            name: cstr2string(raw.pw_name).expect("passwd without name"),
        }
    }
}
pub trait Locate<K> {
    fn locate(key: K) -> IOResult<Self>
    where
        Self: Sized;
}

static PW_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

macro_rules! f {
    ($fnam:ident, $fid:ident, $t:ident, $st:ident) => {
        impl Locate<$t> for $st {
            fn locate(k: $t) -> IOResult<Self> {
                let _guard = PW_LOCK.lock();
                unsafe {
                    let data = $fid(k);
                    if !data.is_null() {
                        Ok($st::from_raw(ptr::read(data as *const _)))
                    } else {
                        Err(IOError::new(
                            ErrorKind::NotFound,
                            format!("No such id: {k}"),
                        ))
                    }
                }
            }
        }

        impl<'a> Locate<&'a str> for $st {
            fn locate(k: &'a str) -> IOResult<Self> {
                let _guard = PW_LOCK.lock();
                if let Ok(id) = k.parse::<$t>() {
                    unsafe {
                        let data = $fid(id);
                        if !data.is_null() {
                            Ok($st::from_raw(ptr::read(data as *const _)))
                        } else {
                            Err(IOError::new(
                                ErrorKind::NotFound,
                                format!("No such id: {id}"),
                            ))
                        }
                    }
                } else {
                    unsafe {
                        let cstring = CString::new(k).unwrap();
                        let data = $fnam(cstring.as_ptr());
                        if !data.is_null() {
                            Ok($st::from_raw(ptr::read(data as *const _)))
                        } else {
                            Err(IOError::new(ErrorKind::NotFound, format!("Not found: {k}")))
                        }
                    }
                }
            }
        }
    };
}

f!(getpwnam, getpwuid, uid_t, Passwd);

pub fn geteuid() -> uid_t {
    unsafe { libc::geteuid() }
}

#[inline]
pub fn uid2usr(id: uid_t) -> IOResult<String> {
    Passwd::locate(id).map(|p| p.name)
}

pub fn get_username() -> io::Result<OsString> {
    uid2usr(geteuid()).map(Into::into)
}
