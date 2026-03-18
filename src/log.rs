#[macro_export]
macro_rules! log_print {
    ($($arg:tt)*) => {
        #[cfg(not(target_arch = "wasm32"))]
        print!($($arg)*);

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!($($arg)*).into());  // wasm always newlines
    }
}

#[macro_export]
macro_rules! log_println {
    ($($arg:tt)*) => {
        #[cfg(not(target_arch = "wasm32"))]
        println!($($arg)*);

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!($($arg)*).into());
    }
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!($($arg)*);

        #[cfg(target_arch = "wasm32")]
        web_sys::console::error_1(&format!($($arg)*).into());
    }
}
