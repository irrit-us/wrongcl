use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::client::{Tunnel, TunnelReader, TunnelWriter};
use crate::endpoint::FragmentOptions;

pub(crate) fn wrap(inner: Box<dyn Tunnel>, opts: &FragmentOptions) -> Box<dyn Tunnel> {
    Box::new(FragmentTunnel {
        inner,
        opts: opts.clone(),
        write_count: Arc::new(Mutex::new(0)),
    })
}

struct FragmentTunnel {
    inner: Box<dyn Tunnel>,
    opts: FragmentOptions,
    write_count: Arc<Mutex<u64>>,
}

struct FragmentWriteHalf {
    inner: Box<dyn TunnelWriter>,
    opts: FragmentOptions,
    write_count: Arc<Mutex<u64>>,
}

fn write_fragmented(
    inner: &mut dyn Write,
    opts: &FragmentOptions,
    write_count: &Arc<Mutex<u64>>,
    buf: &[u8],
) -> io::Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }

    let count = {
        let mut guard = write_count
            .lock()
            .map_err(|_| io::Error::other("fragment write counter poisoned"))?;
        *guard += 1;
        *guard
    };
    if count < opts.packets_from || count > opts.packets_to {
        return inner.write(buf);
    }

    let mut from = 0;
    while from < buf.len() {
        let chunk_len = next_inclusive(opts.length_min, opts.length_max).min(buf.len() - from);
        inner.write_all(&buf[from..from + chunk_len])?;
        from += chunk_len;
        if from < buf.len() {
            let delay_ms = next_inclusive_u64(opts.delay_min_ms, opts.delay_max_ms);
            if delay_ms > 0 {
                thread::sleep(Duration::from_millis(delay_ms));
            }
        }
    }
    Ok(buf.len())
}

fn next_inclusive(min: usize, max: usize) -> usize {
    if min == max {
        min
    } else {
        rand::thread_rng().gen_range(min..=max)
    }
}

fn next_inclusive_u64(min: u64, max: u64) -> u64 {
    if min == max {
        min
    } else {
        rand::thread_rng().gen_range(min..=max)
    }
}

impl Read for FragmentTunnel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Write for FragmentTunnel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_fragmented(&mut self.inner, &self.opts, &self.write_count, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Write for FragmentWriteHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_fragmented(&mut self.inner, &self.opts, &self.write_count, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl TunnelWriter for FragmentWriteHalf {
    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }
}

impl Tunnel for FragmentTunnel {
    fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
        Ok(Box::new(FragmentTunnel {
            inner: self.inner.try_clone_box()?,
            opts: self.opts.clone(),
            write_count: Arc::clone(&self.write_count),
        }))
    }

    fn split_box(self: Box<Self>) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
        let FragmentTunnel {
            inner,
            opts,
            write_count,
        } = *self;
        let (reader, writer) = inner.split_box()?;
        Ok((
            reader,
            Box::new(FragmentWriteHalf {
                inner: writer,
                opts,
                write_count,
            }),
        ))
    }

    fn shutdown_write(&mut self) -> io::Result<()> {
        self.inner.shutdown_write()
    }

    fn set_socket_timeouts(
        &self,
        read: Option<Duration>,
        write: Option<Duration>,
    ) -> io::Result<()> {
        self.inner.set_socket_timeouts(read, write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct RecordingTunnel {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl Read for RecordingTunnel {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Ok(0)
        }
    }

    impl Write for RecordingTunnel {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.lock().unwrap().push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Tunnel for RecordingTunnel {
        fn try_clone_box(&self) -> io::Result<Box<dyn Tunnel>> {
            Ok(Box::new(self.clone()))
        }

        fn split_box(
            self: Box<Self>,
        ) -> io::Result<(Box<dyn TunnelReader>, Box<dyn TunnelWriter>)> {
            Ok((Box::new((*self).clone()), Box::new((*self).clone())))
        }

        fn shutdown_write(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn set_socket_timeouts(
            &self,
            _read: Option<Duration>,
            _write: Option<Duration>,
        ) -> io::Result<()> {
            Ok(())
        }
    }

    impl TunnelWriter for RecordingTunnel {
        fn shutdown_write(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn fragments_selected_write_into_fixed_chunks() {
        let inner = RecordingTunnel::default();
        let writes = Arc::clone(&inner.writes);
        let opts = FragmentOptions {
            length_min: 2,
            length_max: 2,
            delay_min_ms: 0,
            delay_max_ms: 0,
            packets_from: 1,
            packets_to: 1,
        };
        let mut tunnel = wrap(Box::new(inner), &opts);

        tunnel.write_all(b"abcde").unwrap();
        tunnel.write_all(b"fg").unwrap();

        assert_eq!(
            *writes.lock().unwrap(),
            vec![
                b"ab".to_vec(),
                b"cd".to_vec(),
                b"e".to_vec(),
                b"fg".to_vec(),
            ]
        );
    }
}
