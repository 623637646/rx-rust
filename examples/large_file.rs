//! Reading a large file chunk by chunk, each chunk uploaded before the next is read: however slow
//! the upload, the file is never read ahead of it. The checksum of each chunk is computed on a
//! thread of its own, in between, without changing that.
//!
//! ```bash
//! cargo run --example large_file
//! ```

use futures::{Stream, executor::ThreadPool, stream};
use rx_rust::{
    observable::ObservableExt,
    operators::creating::{
        from_try_future::FromTryFuture, from_try_stream_acked::FromTryStreamAcked,
    },
    scheduler::runtime::{futures::ThreadPoolScheduler, tokio::TokioScheduler},
};
use std::{
    hash::{DefaultHasher, Hasher},
    io::{self, Cursor},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::sleep,
};

const CHUNK_SIZE: usize = 256 * 1024;

/// The chunks of `reader`, with their index: each one is read only when the stream is polled.
fn chunks<R: AsyncRead + Unpin>(reader: R) -> impl Stream<Item = io::Result<(usize, Vec<u8>)>> {
    stream::try_unfold((reader, 0), |(mut reader, index)| async move {
        let mut chunk = Vec::with_capacity(CHUNK_SIZE);
        let length = (&mut reader)
            .take(CHUNK_SIZE as u64)
            .read_to_end(&mut chunk)
            .await?;
        if length == 0 {
            return Ok(None);
        }
        println!("read   chunk {index}: {length} bytes");
        Ok(Some(((index, chunk), (reader, index + 1))))
    })
}

/// A chunk as a multipart upload takes it: its place in the file, and a checksum to verify it.
struct Part {
    index: usize,
    checksum: u64,
    bytes: Vec<u8>,
}

/// A fingerprint of `bytes`. A real upload would take the checksum its service asks for, such as
/// an MD5 or a CRC32; `DefaultHasher` is enough to show where it is computed.
fn checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

/// A slow consumer: an upload that takes 100 ms per part.
async fn upload(part: Part) -> io::Result<usize> {
    println!(
        "upload chunk {}: checksum {:#018x}",
        part.index, part.checksum
    );
    sleep(Duration::from_millis(100)).await;
    println!("upload chunk {}: done", part.index);
    Ok(part.bytes.len())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Four full chunks and a short one, of bytes that differ from one chunk to the next. A reader
    // in memory stands in for the file: a real program passes `File::open(path).await?`.
    let content = (0..4 * CHUNK_SIZE + 1000)
        .map(|i| (i % 251) as u8)
        .collect::<Vec<_>>();
    let file = Cursor::new(content);

    let scheduler = TokioScheduler::current();
    let upload_scheduler = scheduler.clone();
    let checksum_scheduler = ThreadPoolScheduler::from_pool(
        ThreadPool::builder()
            .pool_size(1)
            .name_prefix("checksum")
            .create()?,
    );
    let total = FromTryStreamAcked::new(chunks(file), scheduler)
        // A checksum is CPU work: it runs on a thread of its own, off the runtime's workers. The
        // acknowledgement crosses the thread along with its chunk, so the queue of `observe_on`
        // never holds more than one: the file is still not read ahead. Any operator that hands
        // the items on one by one fits between the source and the upload; one that holds items
        // back until more arrive (`buffer_with_count`, `collect`, `zip`) would hold their
        // acknowledgements, and stall.
        .observe_on(checksum_scheduler)
        .map(|((index, bytes), ack)| {
            println!(
                "hash   chunk {index} on {}",
                std::thread::current().name().unwrap_or_default()
            );
            let part = Part {
                index,
                checksum: checksum(&bytes),
                bytes,
            };
            (part, ack)
        })
        // The upload holds the acknowledgement of its chunk: the next chunk is read once it ends.
        // Without the `hold`, the acknowledgement would be dropped here, and the whole file read
        // ahead. `with_permits(n)` and a `flat_map` would upload `n` chunks at once.
        .concat_map(move |(part, ack)| {
            FromTryFuture::new(upload(part), upload_scheduler.clone()).hold(ack)
        })
        .sum()
        .into_try_future()
        .await?;
    println!("uploaded {} bytes", total.unwrap_or_default());
    Ok(())
}
