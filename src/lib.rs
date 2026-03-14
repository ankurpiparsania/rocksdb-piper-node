#![deny(clippy::all)]

use std::sync::Arc;
use napi::threadsafe_function::{ThreadsafeFunction, ErrorStrategy};
// CORRECTED: Import `Buffer` and `JsObject` from napi's prelude
use napi::bindgen_prelude::{Buffer, JsObject};
use napi_derive::napi;
use rocksdb::{
    BlockBasedOptions, Cache, DBCompressionType, Options, WriteOptions, DB,
};

#[napi(object)]
pub struct RocksDbOptions {
    pub compression_type: Option<String>,
    pub block_cache_size_mb: Option<u32>,
    pub write_buffer_size_mb: Option<u32>,
    pub max_write_buffer_number: Option<i32>,
    pub wal_size_limit_mb: Option<u32>,
    pub wal_ttl_seconds: Option<u64>,
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
            
            opts.set_wal_size_limit_mb(options.wal_size_limit_mb.unwrap_or(0));
            opts.set_wal_ttl_seconds(options.wal_ttl_seconds.unwrap_or(0));
        }

        let db = DB::open(&opts, &path).map_err(|e| napi::Error::from_reason(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }

    #[napi(ts_args_type = "options: any, callback: (err: Error | null) => void")]
    pub fn open(&self, _options: JsObject, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) {
        callback.call(Ok(()), |&mut _| Ok(()));
    }

    // CORRECTED: Used `Buffer` instead of `napi::Buffer` and `JsObject` for options.
    #[napi(ts_args_type = "key: string | Buffer, value: string | Buffer, options: any, callback: (err: Error | null) => void")]
    pub fn put(&self, env: napi::Env, key: napi::Either<String, Buffer>, value: napi::Either<String, Buffer>, options: JsObject, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) -> napi::Result<()> {
        let db = self.db.clone();
        let key_bytes: Vec<u8> = key.into();
        let value_bytes: Vec<u8> = value.into();
        // CORRECTED: How to get a property from a JsObject
        let sync = options.get_named_property::<napi::JsBoolean>("sync")?.get_value()?.unwrap_or(false);

        std::thread::spawn(move || {
            let mut write_opts = WriteOptions::default();
            write_opts.set_sync(sync);
            let result = db.put_opt(&key_bytes, &value_bytes, &write_opts)
                .map_err(|e| napi::Error::from_reason(e.to_string()));
            callback.call(result, |&mut _| Ok(()));
        });
        Ok(())
    }

    // CORRECTED: Used `Buffer` and `JsObject`.
    #[napi(ts_args_type = "key: string | Buffer, options: any, callback: (err: Error | null, value: Buffer | null) => void")]
    pub fn get(&self, key: napi::Either<String, Buffer>, _options: JsObject, callback: ThreadsafeFunction<Option<Buffer>, ErrorStrategy::CalleeHandled>) {
        let db = self.db.clone();
        let key_bytes: Vec<u8> = key.into();
        
        std::thread::spawn(move || {
            match db.get(&key_bytes) {
                Ok(Some(value)) => callback.call(Ok(Some(value.into())), |&mut _| Ok(())),
                Ok(None) => callback.call(Ok(None), |&mut _| Ok(())),
                Err(e) => callback.call(Err(napi::Error::from_reason(e.to_string())), |&mut _| Ok(())),
            }
        });
    }

    // CORRECTED: Used `Buffer` and `JsObject`.
    #[napi(ts_args_type = "key: string | Buffer, options: any, callback: (err: Error | null) => void")]
    pub fn del(&self, env: napi::Env, key: napi::Either<String, Buffer>, options: JsObject, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) -> napi::Result<()> {
        let db = self.db.clone();
        let key_bytes: Vec<u8> = key.into();
        let sync = options.get_named_property::<napi::JsBoolean>("sync")?.get_value()?.unwrap_or(false);
        
        std::thread::spawn(move || {
            let mut write_opts = WriteOptions::default();
            write_opts.set_sync(sync);
            let result = db.delete_opt(&key_bytes, &write_opts)
                .map_err(|e| napi::Error::from_reason(e.to_string()));
            callback.call(result, |&mut _| Ok(()));
        });
        Ok(())
    }
}