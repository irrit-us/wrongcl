use super::*;

pub(super) fn relay(
    client: TcpStream,
    upstream: Box<dyn Tunnel>,
    metrics: &ProxyMetrics,
) -> Result<()> {
    relay_with_initial(client, upstream, metrics, &[])
}

pub(super) fn relay_with_initial(
    mut client: TcpStream,
    upstream: Box<dyn Tunnel>,
    metrics: &ProxyMetrics,
    initial_upload: &[u8],
) -> Result<()> {
    let (mut upstream_reader, mut upstream_writer) = upstream.split_box()?;
    let mut client_writer = client.try_clone()?;
    let download_counter = &metrics.bytes_downloaded;
    let downstream = thread::scope(|scope| {
        let downstream = scope.spawn(move || {
            let _ = copy_counted(&mut upstream_reader, &mut client_writer, download_counter);
            let _ = client_writer.shutdown(Shutdown::Write);
        });

        let upload_result = (|| -> io::Result<u64> {
            if !initial_upload.is_empty() {
                upstream_writer.write_all(initial_upload)?;
                metrics
                    .bytes_uploaded
                    .fetch_add(initial_upload.len() as u64, Ordering::Relaxed);
            }
            copy_counted(&mut client, &mut upstream_writer, &metrics.bytes_uploaded)
        })();
        let _ = upstream_writer.shutdown_write();
        let _ = downstream.join();
        upload_result
    });

    downstream.map(|_| ()).map_err(ClientError::Io)
}

fn copy_counted(
    reader: &mut impl Read,
    writer: &mut impl Write,
    counter: &AtomicU64,
) -> io::Result<u64> {
    let mut buf = [0u8; 16 * 1024];
    let mut total = 0u64;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Ok(total),
            Ok(n) => {
                writer.write_all(&buf[..n])?;
                total += n as u64;
                counter.fetch_add(n as u64, Ordering::Relaxed);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}
