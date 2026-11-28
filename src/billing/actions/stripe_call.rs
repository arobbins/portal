use crate::stripe::StripeError;

pub(super) enum StripeCallError {
    Server(topcoat::Error),
    Stripe(StripeError),
}

pub(super) type StripeCall<T> = Result<T, StripeCallError>;

impl From<topcoat::Error> for StripeCallError {
    fn from(error: topcoat::Error) -> Self {
        Self::Server(error)
    }
}

impl From<sqlx::Error> for StripeCallError {
    fn from(error: sqlx::Error) -> Self {
        Self::Server(error.into())
    }
}

impl From<StripeError> for StripeCallError {
    fn from(error: StripeError) -> Self {
        Self::Stripe(error)
    }
}

pub(super) fn split<T>(call: StripeCall<T>) -> topcoat::Result<Result<T, StripeError>> {
    match call {
        Ok(value) => Ok(Ok(value)),
        Err(StripeCallError::Stripe(error)) => Ok(Err(error)),
        Err(StripeCallError::Server(error)) => Err(error),
    }
}
