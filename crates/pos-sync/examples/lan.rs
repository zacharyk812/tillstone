//! Two-machine demo: devices on the same network find each other by public
//! key (no IP addresses needed) and exchange a signed message.
//!
//! ```sh
//! # Machine A
//! cargo run -p pos-sync --example lan -- id a.key           # prints A's id
//! # Machine B
//! cargo run -p pos-sync --example lan -- id b.key           # prints B's id
//! # Machine A: accept messages from B
//! cargo run -p pos-sync --example lan -- listen a.key <B-id>
//! # Machine B: send to A
//! cargo run -p pos-sync --example lan -- send b.key <A-id> "hello from B"
//! ```

use std::{path::Path, time::Duration};

use anyhow::{Context, Result, bail};
use iroh::{EndpointAddr, PublicKey};
use pos_sync::{DeviceIdentity, Members, Peer, PeerOptions};

fn load_or_create(path: &Path) -> Result<DeviceIdentity> {
    if path.exists() {
        let bytes = std::fs::read(path)?;
        let bytes: [u8; 32] = bytes.as_slice().try_into().context("key file must be 32 bytes")?;
        return Ok(DeviceIdentity::from_secret_bytes(&bytes));
    }
    let identity = DeviceIdentity::generate();
    // The demo stores the key in a plain file; the app uses the platform keystore (§9).
    write_private(path, &identity.secret_bytes())?;
    Ok(identity)
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    f.write_all(bytes)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes)?;
    Ok(())
}

fn parse_id(s: &str) -> Result<PublicKey> {
    s.parse().context("not a valid device id")
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["id", key] => {
            println!("{}", load_or_create(Path::new(key))?.device_id());
        }
        ["listen", key, peer] => {
            let identity = load_or_create(Path::new(key))?;
            let members = Members::default();
            members.add(identity.device_id());
            members.add(parse_id(peer)?);
            let (tx, mut rx) = tokio::sync::mpsc::channel(16);
            let me = Peer::bind(identity, members, PeerOptions::LAN, tx).await?;
            println!("listening as {}", me.device_id());
            while let Some(msg) = rx.recv().await {
                println!(
                    "verified message from {}: {}",
                    msg.from.fmt_short(),
                    String::from_utf8_lossy(&msg.payload)
                );
            }
        }
        ["send", key, to, text] => {
            let identity = load_or_create(Path::new(key))?;
            let to = parse_id(to)?;
            let members = Members::default();
            members.add(identity.device_id());
            members.add(to);
            let (tx, _rx) = tokio::sync::mpsc::channel(1);
            let me = Peer::bind(identity, members, PeerOptions::LAN, tx).await?;
            // Give local discovery a moment to find the other device.
            for attempt in 1..=10 {
                match me.send(EndpointAddr::new(to), text.as_bytes()).await {
                    Ok(()) => {
                        println!("delivered and acknowledged");
                        me.shutdown().await;
                        return Ok(());
                    }
                    Err(e) if attempt < 10 => {
                        eprintln!("attempt {attempt}: {e:#}; retrying");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        _ => bail!("usage: lan id <key> | listen <key> <peer-id> | send <key> <peer-id> <text>"),
    }
    Ok(())
}
