use std::sync::Mutex;

use once_cell::sync::Lazy;

pub static SRC: Lazy<Mutex<Vec<Vec<String>>>> = Lazy::new(|| Mutex::new(vec![]));
pub static FILE_NAMES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));