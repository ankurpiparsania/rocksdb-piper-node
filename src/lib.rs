#![deny(clippy::all)]

use napi_derive::napi;
use rocksdb::{BlockBasedOptions, DBCompressionType, Options, WriteOptions, DB};

#[napi(js_name = "RocksDB")]
pub struct JsRocksDb {
  db: DB,
}

#[napi]
impl JsRocksDb {
  #[napi(constructor)]
  pub fn new(path: String) -> napi::Result<Self> {
    let mut opts = Options::default();
    opts.create_if_missing(true);

    // 1. ZSTD COMPRESSION
    opts.set_compression_type(DBCompressionType::Zstd);

    // 2. 16MB BLOCK CACHE
    let mut block_opts = BlockBasedOptions::default();
    let cache = rocksdb::Cache::new_lru_cache(16 * 1024 * 1024);
    block_opts.set_block_cache(&cache);
    opts.set_block_based_table_factory(&block_opts);

    // 3. 8MB WRITE BUFFER
    opts.set_write_buffer_size(8 * 1024 * 1024);

    // 4. MAX WRITE BUFFERS
    opts.set_max_write_buffer_number(2);

    // 5. WAL LIMITS
    opts.set_wal_size_limit_mb(0);
    opts.set_wal_ttl_seconds(0);

    let db = DB::open(&opts, path).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self { db })
  }

  #[napi]
  pub fn put(&self, key: String, value: String, sync: bool) -> napi::Result<()> {
    let mut write_opts = WriteOptions::default();
    write_opts.set_sync(sync);
    self.db.put_opt(key.as_bytes(), value.as_bytes(), &write_opts)
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  #[napi]
  pub fn get(&self, key: String) -> napi::Result<Option<String>> {
    match self.db.get(key.as_bytes()) {
      Ok(Some(value)) => {
        let s = String::from_utf8(value).map_err(|_| napi::Error::from_reason("Not UTF-8".to_string()))?;
        Ok(Some(s))
      }
      Ok(None) => Ok(None),
      Err(e) => Err(napi::Error::from_reason(e.to_string())),
    }
  }

  #[napi]
  pub fn delete(&self, key: String) -> napi::Result<()> {
    self.db.delete(key.as_bytes()).map_err(|e| napi::Error::from_reason(e.to_string()))
  }
}