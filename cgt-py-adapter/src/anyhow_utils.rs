use anyhow::Context;

/// Report a warning in a style of error
pub fn warn<E>(err: E, msg: &str)
where
    Result<(), E>: Context<(), E>,
{
    let err: Result<(), E> = Err(err);
    let err = err.context(format!("Warning: {}", msg)).err().unwrap();
    eprintln!("{:?}", err);
}
