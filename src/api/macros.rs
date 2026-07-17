//! Delegate macros for bridging async API functions into the synchronous service layer.

/// Delegates to an async function via a blocking Tokio runtime.
macro_rules! delegate {
    ($vis:vis fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty = $path:path) => {
        #[doc = concat!("Delegates to [`", stringify!($path), "`].")]
        #[doc = ""]
        #[doc = "# Errors"]
        #[doc = ""]
        #[doc = "Returns a `QobuzApiError` if not authenticated or the API request fails."]
        $vis fn $name(&self $(, $arg: $ty)*) -> Result<$ret, crate::errors::QobuzApiError> {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on($path(self $(, $arg)*))
        }
    };
}

/// Internal helper: the retry-on-signature-error body shared by retry macros.
macro_rules! retry_body {
    ($path:path, $rt:ident, $self:ident, first: ($($first_args:tt)*), retry: ($($retry_args:tt)*)) => {
        {
            let result = $rt.block_on($path($self, $($first_args)*));
            match result {
                Err(crate::errors::QobuzApiError::ApiErrorResponse { message, .. })
                    if message.contains("Invalid Request Signature") =>
                {
                    tracing::info!(
                        concat!(
                            "Signature invalid for ",
                            stringify!($path),
                            ", refreshing credentials"
                        )
                    );

                    crate::api::auth::refresh_app_credentials($self)?;

                    $rt.block_on($path($self, $($retry_args)*))
                }
                other => other,
            }
        }
    };
}

/// Delegates to an async function with automatic credential refresh on signature errors.
macro_rules! delegate_with_retry {
    ($vis:vis fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty = $path:path) => {
        delegate_with_retry!(
            @inner $vis fn $name -> $ret = $path,
            fn_args: ($(, $arg: $ty)*),
            first: ($($arg),*),
            retry: ($($arg),*)
        );
    };
    ($vis:vis fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty = $path:path, cancel: $cancel_ty:ty) => {
        delegate_with_retry!(
            @inner $vis fn $name -> $ret = $path,
            fn_args: ($(, $arg: $ty)*, cancel: $cancel_ty),
            first: ($($arg),*, cancel.clone()),
            retry: ($($arg),*, cancel)
        );
    };
    (@inner $vis:vis fn $name:ident -> $ret:ty = $path:path, fn_args: ($($fn_args:tt)*), first: ($($first:tt)*), retry: ($($retry:tt)*)) => {
        #[doc = concat!("Delegates to [`", stringify!($path), "`] with auto-refresh on signature errors.")]
        #[doc = ""]
        #[doc = "# Errors"]
        #[doc = ""]
        #[doc = "Returns a `QobuzApiError` if not authenticated, the API request fails, or refresh fails."]
        $vis fn $name(&mut self $($fn_args)*) -> Result<$ret, crate::errors::QobuzApiError> {
            let rt = tokio::runtime::Runtime::new()?;
            retry_body!($path, rt, self, first: ($($first)*), retry: ($($retry)*))
        }
    };
}

/// Delegates to an async function with auto-refresh and a cancel parameter.
macro_rules! delegate_with_retry_cancellable {
    ($vis:vis fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty = $path:path, cancel: $cancel_ty:ty) => {
        delegate_with_retry!($vis fn $name($($arg: $ty),*) -> $ret = $path, cancel: $cancel_ty);
    };
}
