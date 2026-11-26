use std::fs::File;
use std::io::Read;
use std::path::Path;

use tokio::runtime::Builder;

use super::connect_session::{connect_session, connect_tcp, disconnect, within};
use super::scp_channel::ScpChannel;
use crate::domain::scp::build_file_header::build_file_header;
use crate::domain::scp::parse_file_header::parse_file_header;
use crate::domain::scp::quote_remote_path::quote_remote_path;
use crate::{DeviceParams, Result};

const CHUNK_SIZE: usize = 32 * 1024;

/// Copies one local file to `remote` over a separate SSH session, like netmiko's `file_transfer(direction="put")`.
pub fn scp_upload(params: &DeviceParams, local: &Path, remote: &str) -> Result<()> {
    let file = File::open(local)?;
    let size = file.metadata()?.len();
    let header = build_file_header(size, &local.file_name().unwrap_or_default().to_string_lossy())?;
    // Never send more than the header announced, even if the file grows meanwhile.
    let mut content = file.take(size);
    run_scp(params, &format!("scp -t {}", quote_remote_path(remote)), async |channel| {
        channel.expect_ack().await?;
        channel.send(header.as_bytes()).await?;
        channel.expect_ack().await?;
        let mut chunk = vec![0; CHUNK_SIZE];
        loop {
            let read = content.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            channel.send(&chunk[..read]).await?;
        }
        channel.send_ack().await?;
        channel.expect_ack().await
    })
}

/// Copies one remote file to `local` over a separate SSH session, like netmiko's `file_transfer(direction="get")`.
pub fn scp_download(params: &DeviceParams, remote: &str, local: &Path) -> Result<()> {
    run_scp(params, &format!("scp -f {}", quote_remote_path(remote)), async |channel| {
        channel.send_ack().await?;
        let size = parse_file_header(&channel.read_line().await?)?;
        // Created only once the remote confirmed the file exists, so a typo does not wipe an existing local copy.
        let mut file = File::create(local)?;
        channel.send_ack().await?;
        channel.read_into(size, &mut file).await?;
        channel.expect_ack().await?;
        channel.send_ack().await
    })
}

fn run_scp(params: &DeviceParams, command: &str, transfer: impl AsyncFnOnce(&mut ScpChannel) -> Result<()>) -> Result<()> {
    let runtime = Builder::new_current_thread().enable_all().build()?;
    let username = format!("{}{}", params.username, params.device_type.profile().username_suffix);
    runtime.block_on(async {
        let tcp = connect_tcp(params).await?;
        let mut session = within(params.timeout, connect_session(tcp, params, &username)).await??;
        let result = async {
            let mut channel = ScpChannel::open(&session, command, params.timeout).await?;
            transfer(&mut channel).await?;
            channel.finish().await;
            Ok(())
        }
        .await;
        let _ = within(params.timeout, disconnect(&mut session)).await;
        result
    })
}
