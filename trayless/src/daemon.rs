use std::{collections::HashMap, sync::{Arc, Mutex}};
use futures_util::{StreamExt};
use zbus::{Connection, fdo::DBusProxy};
use anyhow::{Result, anyhow};
use libtrayless::watcher::StatusNotifierWatcherService;

pub async fn cmd_daemon(_cli_args: crate::cli::Cli, _cmd_args: ()) -> Result<()> {
    // TODO fail if there is a watcher already

    let conn = Connection::session().await?;
    let items = Arc::new(Mutex::new(HashMap::new()));
    let watcher = StatusNotifierWatcherService {
        items: items.clone(),
        hosts: Vec::new(),
    };

    conn.object_server()
        .at("/StatusNotifierWatcher", watcher)
        .await?;

    match conn.request_name_with_flags("org.kde.StatusNotifierWatcher", zbus::fdo::RequestNameFlags::DoNotQueue.into()).await {
        Ok(_) => {},
        // NOTE replacing the error to make it a bit more clear what is happening
        Err(zbus::Error::NameTaken) => return Err(anyhow!("StatusNotifierWatcher is already registered")),
        Err(x) => return Err(x.into()),
    };

    let conn = conn.clone();
    tokio::spawn(async move {
        match daemon_listen_disconnect(conn, items).await {
            Ok(_) => {},
            Err(x) => {
                // TODO panic here
                println!("error listening to disconnects {x}");
            }
        }
    });

    println!("Daemon running... Press Ctrl+C to stop.");
    std::future::pending::<()>().await;

    Ok(())
}

/// Listens for when notifier items disconnect
async fn daemon_listen_disconnect(
    conn: Connection,
    items: Arc<Mutex<HashMap<String, String>>>
) -> Result<()> {
    let proxy = DBusProxy::new(&conn).await?;

    let Ok(mut stream) = proxy.receive_name_owner_changed().await else {
        // TODO better erorr message
        return Err(anyhow!("could not get the stream"));
    };

    while let Some(signal) = stream.next().await {
        let Ok(args) = signal.args() else { continue };

        // TODO remove StatusNotifierHost instances as well
        if args.new_owner.is_none() {
            if let Some(name) = items.lock().unwrap().remove(args.name.as_str()) {
                println!("- {name} StatusNotifierItem");
            }
        }
    }

    Ok(())
}

