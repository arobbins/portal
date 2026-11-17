use std::convert::Infallible;

use crate::action::{ActionResult, Caller, authorize};
use crate::admin::queries::{LogEntry, query_list_admin_log};
use crate::env::Env;
use crate::permission::Permission;

pub const LOG_ENTRIES_SHOWN: i64 = 100;

pub async fn list_admin_log(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<Vec<LogEntry>, Infallible> {
    authorize!(caller.require(Permission::AdministerUsers));
    let entries = query_list_admin_log(LOG_ENTRIES_SHOWN, env.db()).await?;

    Ok(Ok(entries))
}
