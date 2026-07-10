use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zlf_storage::StorageRecord;

pub const BULK_PACK_VERSION: u32 = 1;
pub const MANIFEST_FILE: &str = "manifest.bin";
pub const RECORDS_FILE: &str = "records.bin";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkPackManifest {
    pub format_version: u32,
    pub storage_key_version: u32,
    pub source_checksums: BTreeMap<String, u64>,
    pub fact_counts: BTreeMap<String, u64>,
    pub record_count: u64,
    pub records_checksum: u64,
    pub complete: bool,
}

pub(crate) struct RecordWriter {
    writer: BufWriter<File>,
    pub count: u64,
    pub checksum: u64,
}

impl RecordWriter {
    pub fn create(path: &Path) -> io::Result<Self> {
        Ok(Self {
            writer: BufWriter::new(File::create(path)?),
            count: 0,
            checksum: checksum_seed(),
        })
    }

    pub fn write(&mut self, record: &StorageRecord) -> io::Result<()> {
        let bytes = bincode::serialize(record).map_err(io::Error::other)?;
        let length = u32::try_from(bytes.len())
            .map_err(|_| io::Error::other("bulk record exceeds u32 length"))?;
        self.writer.write_all(&length.to_le_bytes())?;
        self.writer.write_all(&bytes)?;
        self.checksum = checksum_update(self.checksum, &length.to_le_bytes());
        self.checksum = checksum_update(self.checksum, &bytes);
        self.count += 1;
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<(u64, u64)> {
        self.writer.flush()?;
        Ok((self.count, self.checksum))
    }
}

pub(crate) struct RecordReader {
    reader: BufReader<File>,
    checksum: u64,
}

impl RecordReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self {
            reader: BufReader::new(File::open(path)?),
            checksum: checksum_seed(),
        })
    }

    pub fn next_record(&mut self) -> io::Result<Option<StorageRecord>> {
        let mut length_bytes = [0_u8; 4];
        match self.reader.read_exact(&mut length_bytes) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(error) => return Err(error),
        }
        let length = u32::from_le_bytes(length_bytes) as usize;
        let mut bytes = vec![0; length];
        self.reader.read_exact(&mut bytes)?;
        self.checksum = checksum_update(self.checksum, &length_bytes);
        self.checksum = checksum_update(self.checksum, &bytes);
        bincode::deserialize(&bytes)
            .map(Some)
            .map_err(io::Error::other)
    }

    pub fn checksum(&self) -> u64 {
        self.checksum
    }
}

pub(crate) fn write_manifest(path: &Path, manifest: &BulkPackManifest) -> io::Result<()> {
    let bytes = bincode::serialize(manifest).map_err(io::Error::other)?;
    std::fs::write(path.join(MANIFEST_FILE), bytes)
}

pub(crate) fn read_manifest(path: &Path) -> io::Result<BulkPackManifest> {
    let bytes = std::fs::read(path.join(MANIFEST_FILE))?;
    bincode::deserialize(&bytes).map_err(io::Error::other)
}

pub(crate) fn checksum_file(path: &Path) -> io::Result<u64> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut checksum = checksum_seed();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(checksum);
        }
        checksum = checksum_update(checksum, &buffer[..count]);
    }
}

fn checksum_seed() -> u64 {
    0xcbf2_9ce4_8422_2325
}

fn checksum_update(mut checksum: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        checksum = (checksum ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    checksum
}
