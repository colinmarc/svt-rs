#[derive(Debug, Copy, Clone)]
#[allow(missing_docs)]
pub enum Error {
    InsufficientResources,
    Undefined,
    InvalidComponent,
    BadParameter,
    DestroyThreadFailed,
    SemaphoreUnresponsive,
    DestroySemaphoreFailed,
    CreateMutexFailed,
    MutexUnresponsive,
    DestroyMutexFailed,
    Unknown(i32),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::InsufficientResources => "EB_ErrorInsufficientResources",
            Error::Undefined => "EB_ErrorUndefined",
            Error::InvalidComponent => "EB_ErrorInvalidComponent",
            Error::BadParameter => "EB_ErrorBadParameter",
            Error::DestroyThreadFailed => "EB_ErrorDestroyThreadFailed",
            Error::SemaphoreUnresponsive => "EB_ErrorSemaphoreUnresponsive",
            Error::DestroySemaphoreFailed => "EB_ErrorDestroySemaphoreFailed",
            Error::CreateMutexFailed => "EB_ErrorCreateMutexFailed",
            Error::MutexUnresponsive => "EB_ErrorMutexUnresponsive",
            Error::DestroyMutexFailed => "EB_ErrorDestroyMutexFailed",
            Error::Unknown(_) => "Unknown error",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InsufficientResources => write!(f, "EB_ErrorInsufficientResources"),
            Error::Undefined => write!(f, "EB_ErrorUndefined"),
            Error::InvalidComponent => write!(f, "EB_ErrorInvalidComponent"),
            Error::BadParameter => write!(f, "EB_ErrorBadParameter"),
            Error::DestroyThreadFailed => write!(f, "EB_ErrorDestroyThreadFailed"),
            Error::SemaphoreUnresponsive => write!(f, "EB_ErrorSemaphoreUnresponsive"),
            Error::DestroySemaphoreFailed => write!(f, "EB_ErrorDestroySemaphoreFailed"),
            Error::CreateMutexFailed => write!(f, "EB_ErrorCreateMutexFailed"),
            Error::MutexUnresponsive => write!(f, "EB_ErrorMutexUnresponsive"),
            Error::DestroyMutexFailed => write!(f, "EB_ErrorDestroyMutexFailed"),
            Error::Unknown(code) => write!(f, "Unknown error code: {}", code),
        }
    }
}
