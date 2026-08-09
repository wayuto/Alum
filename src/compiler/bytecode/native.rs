use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::OnceLock;

const RTLD_NOW: c_int = 2;
const RTLD_LOCAL: c_int = 0;

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeKind {
    Int,
    Float,
    Bool,
    Str,
}

#[derive(Debug, Clone, Copy)]
pub struct NativeSig {
    pub params: &'static [NativeKind],
    pub ret: NativeKind,
}

#[derive(Debug, Clone, Copy)]
pub struct NativeEntry {
    pub ptr: usize,
    pub sig: NativeSig,
}

pub struct NativeTable {
    pub entries: HashMap<String, NativeEntry>,
    handles: Vec<*mut c_void>,
}

impl NativeTable {
    pub fn open(libs: &[String]) -> Result<NativeTable, String> {
        ffi().ok_or_else(|| {
            "libffi is not available (needed for compile-time native calls)".to_string()
        })?;
        let mut table = NativeTable {
            entries: HashMap::new(),
            handles: Vec::new(),
        };
        for lib in libs {
            let lib_name = CString::new(lib.as_str())
                .map_err(|_| format!("invalid native library path '{lib}'"))?;
            let handle = unsafe { dlopen(lib_name.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
            if handle.is_null() {
                return Err(format!(
                    "cannot open native library '{lib}' for compile-time evaluation"
                ));
            }
            table.handles.push(handle);
        }
        Ok(table)
    }

    pub fn resolve(&mut self, name: &str, sig: NativeSig) -> Option<NativeEntry> {
        let symbol = CString::new(name).ok()?;
        for handle in &self.handles {
            let ptr = unsafe { dlsym(*handle, symbol.as_ptr()) };
            if !ptr.is_null() {
                let entry = NativeEntry {
                    ptr: ptr as usize,
                    sig,
                };
                self.entries.insert(name.to_string(), entry);
                return Some(entry);
            }
        }
        None
    }
}

impl Drop for NativeTable {
    fn drop(&mut self) {
        for handle in &self.handles {
            unsafe {
                dlclose(*handle);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Ffi {
    prep_cif:
        unsafe extern "C" fn(*mut u8, c_int, u32, *const c_void, *const *const c_void) -> c_int,
    call: unsafe extern "C" fn(*const u8, *const c_void, *mut c_void, *const *const c_void),
    t_sint32: *const c_void,
    t_double: *const c_void,
    t_pointer: *const c_void,
}

unsafe impl Sync for Ffi {}
unsafe impl Send for Ffi {}

fn ffi() -> Option<&'static Ffi> {
    static FFI: OnceLock<Option<Ffi>> = OnceLock::new();
    FFI.get_or_init(load_ffi).as_ref()
}

fn load_ffi() -> Option<Ffi> {
    for name in ["libffi.so.8", "libffi.so.7", "libffi.so.6", "libffi.so"] {
        let Some(handle) = open_lib(name) else {
            continue;
        };
        let Some(prep_cif) = f_sym_typed::<
            unsafe extern "C" fn(*mut u8, c_int, u32, *const c_void, *const *const c_void) -> c_int,
        >(handle, "ffi_prep_cif") else {
            continue;
        };
        let Some(call) = f_sym_typed::<
            unsafe extern "C" fn(*const u8, *const c_void, *mut c_void, *const *const c_void),
        >(handle, "ffi_call") else {
            continue;
        };
        let Some(t_sint32) = f_sym(handle, "ffi_type_sint32") else {
            continue;
        };
        let Some(t_double) = f_sym(handle, "ffi_type_double") else {
            continue;
        };
        let Some(t_pointer) = f_sym(handle, "ffi_type_pointer") else {
            continue;
        };
        return Some(Ffi {
            prep_cif,
            call,
            t_sint32: t_sint32 as *const c_void,
            t_double: t_double as *const c_void,
            t_pointer: t_pointer as *const c_void,
        });
    }
    None
}

fn ffi_type_of(kind: NativeKind, f: &Ffi) -> *const c_void {
    match kind {
        NativeKind::Int | NativeKind::Bool => f.t_sint32,
        NativeKind::Float => f.t_double,
        NativeKind::Str => f.t_pointer,
    }
}

pub fn call_native(
    entry: &NativeEntry,
    args: &[crate::compiler::bytecode::Value],
) -> Option<crate::compiler::bytecode::Value> {
    use crate::compiler::bytecode::Value;

    let ffi = ffi()?;
    let params = entry.sig.params;
    if params.len() != args.len() {
        return None;
    }

    let mut storage: Vec<u64> = Vec::with_capacity(params.len());
    let mut avalues: Vec<*const c_void> = Vec::with_capacity(params.len());
    let mut strings: Vec<CString> = Vec::new();

    for (kind, value) in params.iter().zip(args.iter()) {
        let slot: u64 = match kind {
            NativeKind::Int => match value {
                Value::Int(i) => *i as u64,
                Value::Float(f) => *f as u64,
                Value::Bool(b) => (*b as u64) & 1,
                _ => return None,
            },
            NativeKind::Bool => match value {
                Value::Bool(b) => (*b as u64) & 1,
                Value::Int(i) => ((*i != 0) as u64) & 1,
                _ => return None,
            },
            NativeKind::Float => match value {
                Value::Float(f) => f.to_bits(),
                Value::Int(i) => (*i as f64).to_bits(),
                _ => return None,
            },
            NativeKind::Str => match value {
                Value::Str(s) => {
                    let c = CString::new(s.clone()).ok()?;
                    let p = c.as_ptr() as u64;
                    strings.push(c);
                    p
                }
                _ => return None,
            },
        };
        let idx = storage.len();
        storage.push(slot);
        avalues.push(unsafe { storage.as_ptr().add(idx) as *const c_void });
    }

    let mut cif_buf = vec![0u8; 256];
    let arg_types: Vec<*const c_void> = params.iter().map(|k| ffi_type_of(*k, ffi)).collect();
    let ret_type = ffi_type_of(entry.sig.ret, ffi);
    let mut rvalue: u64 = 0;

    unsafe {
        let status = (ffi.prep_cif)(
            cif_buf.as_mut_ptr(),
            2,
            params.len() as u32,
            ret_type,
            if arg_types.is_empty() {
                std::ptr::null()
            } else {
                arg_types.as_ptr()
            },
        );
        if status != 0 {
            return None;
        }
        (ffi.call)(
            cif_buf.as_ptr(),
            entry.ptr as *const c_void,
            &mut rvalue as *mut u64 as *mut c_void,
            avalues.as_ptr(),
        );
    }

    let result = match entry.sig.ret {
        NativeKind::Int => {
            let v = rvalue as i32;
            Value::Int(v as i64)
        }
        NativeKind::Bool => Value::Bool((rvalue as i32) != 0),
        NativeKind::Float => Value::Float(f64::from_bits(rvalue)),
        NativeKind::Str => {
            let p = rvalue as *const c_char;
            if p.is_null() {
                Value::Str(String::new())
            } else {
                let s = unsafe { CStr::from_ptr(p) };
                Value::Str(s.to_string_lossy().into_owned())
            }
        }
    };
    Some(result)
}

fn open_lib(name: &str) -> Option<*mut c_void> {
    let n = CString::new(name).ok()?;
    let h = unsafe { dlopen(n.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
    if h.is_null() { None } else { Some(h) }
}

fn f_sym(handle: *mut c_void, name: &str) -> Option<*mut c_void> {
    let n = CString::new(name).ok()?;
    let p = unsafe { dlsym(handle, n.as_ptr()) };
    if p.is_null() { None } else { Some(p) }
}

fn f_sym_typed<T: Copy>(handle: *mut c_void, name: &str) -> Option<T> {
    f_sym(handle, name).map(|p| unsafe { std::mem::transmute_copy::<*mut c_void, T>(&p) })
}
