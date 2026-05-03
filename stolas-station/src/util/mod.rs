pub mod shutdown;

#[macro_export]
macro_rules! do_once {
    {$($code:tt)*} => {
        {
            static _ONCE: ::std::sync::atomic::AtomicBool = ::std::sync::atomic::AtomicBool::new(false);
            if !_ONCE.fetch_or(true, ::std::sync::atomic::Ordering::Relaxed) {
                $($code)*
            }
        }
    };
}

#[macro_export]
macro_rules! log_error_once {
    ($result:expr) => {{
        let result = $result;
        if let Err(error) = &result {
            use crate::do_once;

            do_once! {
                tracing::error!("{error}");
            }
        }
        result
    }};
}

pub fn linear_to_db(value: f32) -> f32 {
    10.0 * value.log10()
}
