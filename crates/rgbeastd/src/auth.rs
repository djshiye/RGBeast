//! polkit authorisation for every call that changes hardware or stored state.

use std::collections::HashMap;

use zbus::message::Header;
use zbus_polkit::policykit1::{AuthorityProxy, CheckAuthorizationFlags, Subject};

pub const ACTION_CONTROL: &str = "io.github.djshiye.rgbeast.control";

/// Ask polkit whether the caller may perform `action`. On the session bus
/// (development) there is no polkit subject for the caller, so it is skipped.
pub async fn check(
    conn: &zbus::Connection,
    hdr: &Header<'_>,
    action: &str,
    system_bus: bool,
) -> zbus::fdo::Result<()> {
    if !system_bus {
        return Ok(());
    }
    let subject = Subject::new_for_message_header(hdr)
        .map_err(|e| zbus::fdo::Error::AuthFailed(format!("cannot identify caller: {e}")))?;
    let authority = AuthorityProxy::new(conn)
        .await
        .map_err(|e| zbus::fdo::Error::Failed(format!("polkit unavailable: {e}")))?;
    let result = authority
        .check_authorization(
            &subject,
            action,
            &HashMap::new(),
            CheckAuthorizationFlags::AllowUserInteraction.into(),
            "",
        )
        .await
        .map_err(|e| zbus::fdo::Error::Failed(format!("polkit check failed: {e}")))?;
    if result.is_authorized {
        Ok(())
    } else {
        Err(zbus::fdo::Error::AccessDenied(
            "not authorised to control lighting".into(),
        ))
    }
}
