mod context;
mod middleware;

pub use context::{app, config, env, globals, redisplay, session};
pub use middleware::{
    CanonicalHost, attach_remote_addr, inject_globals, redirect_to_canonical_host,
    run_to_completion,
};
