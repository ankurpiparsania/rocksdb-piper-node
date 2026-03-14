#![deny(clippy::all)]

use std::sync::Arc;

use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::bindgen_prelude::{Buffer, Either, JsObject};
use napi_derive::napi;
use rocksdb::{BlockBasedOptions, Cache, DBCompressionType, Options, WriteOptions, DB};

#[napi(object)]
pub struct RocksDbOptions {
  pub compression_type: Option<String>,
  pub block_cache_size_mb: Option<u32>,
  pub write_buffer_size_mb: Option<u32>,
  pub max_write_buffer_number: Option<i32>,
  pub wal_size_limit_mb: Option<u32>,
  pub wal_ttl_seconds: Option<u32>,
}

#[napi(js_name = "RocksDB")]
pub struct JsRocksDb {
  db: Arc<DB>,
}

#[napi]
impl JsRocksDb {
  #[napi(constructor)]
  pub fn new(path: String, options: Option<RocksDbOptions>) -> napi::Result<Self> {
    let mut opts = Options::default();
    opts.create_if_missing(true);

    if let Some(options) = options {
      if let Some(compression) = options.compression_type {
        let db_compression = match compression.to_lowercase().as_str() {
          "zstd" => DBCompressionType::Zstd,
          "lz4" => DBCompressionType::Lz4,
          "none" => DBCompressionType::None,
          _ => DBCompressionType::Snappy,
        };
        opts.set_compression_type(db_compression);
      }

      let cache_size = options.block_cache_size_mb.unwrap_or(16) as usize * 1024 * 1024;
      let mut block_opts = BlockBasedOptions::default();
      let cache = Cache::new_lru_cache(cache_size);
      block_opts.set_block_cache(&cache);
      opts.set_block_based_table_factory(&block_opts);

      let buffer_size = options.write_buffer_size_mb.unwrap_or(8) as usize * 1024 * 1024;
      opts.set_write_buffer_size(buffer_size);

      let max_buffers = options.max_write_buffer_number.unwrap_or(2);
      opts.set_max_write_buffer_number(max_buffers);

      opts.set_wal_size_limit_mb(options.wal_size_limit_mb.unwrap_or(0).into());
      opts.set_wal_ttl_seconds(options.wal_ttl_seconds.unwrap_or(0).into());
    }

    let db = DB::open(&opts, &path).map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(Self { db: Arc::new(db) })
  }

  #[napi(ts_args_type = "options: any, callback: (err: Error | null) => void")]
  pub fn open(&self, _options: JsObject, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) {
    callback.call(Ok(()), ThreadsafeFunctionCallMode::Blocking);
  }

  #[napi(ts_args_type = "key: string | Buffer, value: string | Buffer, options: any, callback: (err: Error | null) => void")]
  pub fn put(
    &self,
    key: Either<String, Buffer>,
    value: Either<String, Buffer>,
    options: JsObject,
    callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>,
  ) -> napi::Result<()> {
    let db = self.db.clone();
    let key_bytes = either_to_vec(key);
    let value_bytes = either_to_vec(value);
    let sync = options.get::<_, bool>("sync")?.unwrap_or(false);

    std::thread::spawn(move || {
      let mut write_opts = WriteOptions::default();
      write_opts.set_sync(sync);
      let result = db
        .put_opt(&key_bytes, &value_bytes, &write_opts)
        .map_err(|e| napi::Error::from_reason(e.to_string()));
      callback.call(result, ThreadsafeFunctionCallMode::Blocking);
    });

    Ok(())
  }

  #[napi(ts_args_type = "key: string | Buffer, options: any, callback: (err: Error | null, value: Buffer | null) => void")]
  pub fn get(
    &self,
    key: Either<String, Buffer>,
    _options: JsObject,
    callback: ThreadsafeFunction<Option<Buffer>, ErrorStrategy::CalleeHandled>,
  ) {
    let db = self.db.clone();
    let key_bytes = either_to_vec(key);

    std::thread::spawn(move || match db.get(&key_bytes) {
      Ok(Some(value)) => callback.call(Ok(Some(value.into())), ThreadsafeFunctionCallMode::Blocking),
      Ok(None) => callback.call(Ok(None), ThreadsafeFunctionCallMode::Blocking),
      Err(e) => callback.call(
        Err(napi::Error::from_reason(e.to_string())),
        ThreadsafeFunctionCallMode::Blocking,
      ),
    });
  }

  #[napi(ts_args_type = "key: string | Buffer, options: any, callback: (err: Error | null) => void")]
  pub fn del(
    &self,
    key: Either<String, Buffer>,
    options: JsObject,
    callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>,
  ) -> napi::Result<()> {
    let db = self.db.clone();
    let key_bytes = either_to_vec(key);
    let sync = options.get::<_, bool>("sync")?.unwrap_or(false);

    std::thread::spawn(move || {
      let mut write_opts = WriteOptions::default();
      write_opts.set_sync(sync);
      let result = db
        .delete_opt(&key_bytes, &write_opts)
        .map_err(|e| napi::Error::from_reason(e.to_string()));
      callback.call(result, ThreadsafeFunctionCallMode::Blocking);
    });

    Ok(())
  }
}

fn either_to_vec(e: Either<String, Buffer>) -> Vec<u8> {
  match e {
    Either::A(s) => s.into_bytes(),
    Either::B(b) => b.into(),
  }
}
