#![deny(clippy::all)]

use std::sync::Arc;
use napi::threadsafe_function::{ThreadsafeFunction, ErrorStrategy};
use napi_derive::napi;
use rocksdb::{
    BlockBasedOptions, Cache, DBCompressionType, Options, WriteOptions, DB,
};

// This struct will be passed from JavaScript to configure RocksDB
#[napi(object)]
pub struct RocksDbOptions {
    pub compression_type: Option<String>,
    pub block_cache_size_mb: Option<u32>,
    pub write_buffer_size_mb: Option<u32>,
    pub max_write_buffer_number: Option<i32>,
    pub wal_size_limit_mb: Option<u32>,
    pub wal_ttl_seconds: Option<u64>,
}

// The main database class, now holding a thread-safe reference to the DB
#[napi(js_name = "RocksDB")]
pub struct JsRocksDb {
    db: Arc<DB>,
}

#[napi]
impl JsRocksDb {
    // The constructor now accepts the path and an optional config object
    #[napi(constructor)]
    pub fn new(path: String, options: Option<RocksDbOptions>) -> napi::Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        // Apply configurations from the options object, with sensible defaults
        if let Some(options) = options {
            // 1. ZSTD COMPRESSION (Now configurable)
            if let Some(compression) = options.compression_type {
                let db_compression = match compression.to_lowercase().as_str() {
                    "zstd" => DBCompressionType::Zstd,
                    "lz4" => DBCompressionType::Lz4,
                    "none" => DBCompressionType::None,
                    _ => DBCompressionType::Snappy, // Default
                };
                opts.set_compression_type(db_compression);
            }

            // 2. BLOCK CACHE (Now configurable)
            let cache_size = options.block_cache_size_mb.unwrap_or(16) as usize * 1024 * 1024;
            let mut block_opts = BlockBasedOptions::default();
            let cache = Cache::new_lru_cache(cache_size);
            block_opts.set_block_cache(&cache);
            opts.set_block_based_table_factory(&block_opts);

            // 3. WRITE BUFFER SIZE (Now configurable)
            let buffer_size = options.write_buffer_size_mb.unwrap_or(8) as usize * 1024 * 1024;
            opts.set_write_buffer_size(buffer_size);

            // 4. MAX WRITE BUFFERS (Now configurable)
            let max_buffers = options.max_write_buffer_number.unwrap_or(2);
            opts.set_max_write_buffer_number(max_buffers);
            
            // 5. WAL LIMITS (Now configurable)
            opts.set_wal_size_limit_mb(options.wal_size_limit_mb.unwrap_or(0));
            opts.set_wal_ttl_seconds(options.wal_ttl_seconds.unwrap_or(0));
        }

        let db = DB::open(&opts, &path).map_err(|e| napi::Error::from_reason(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }

    // --- LEVELDOWN-COMPATIBLE ASYNCHRONOUS METHODS ---

    #[napi(ts_args_type = "options: any, callback: (err: Error | null) => void")]
    pub fn open(&self, _options: serde_json::Value, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) {
        // In our case, the DB is already open. We just call back to satisfy the API.
        callback.call(Ok(()), |&mut _| Ok(()));
    }

    #[napi(ts_args_type = "key: string | Buffer, value: string | Buffer, options: any, callback: (err: Error | null) => void")]
    pub fn put(&self, key: napi::Either<String, napi::Buffer>, value: napi::Either<String, napi::Buffer>, options: serde_json::Value, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) {
        let db = self.db.clone();
        let key_bytes = either_to_vec(key);
        let value_bytes = either_to_vec(value);
        let sync = options.get("sync").and_then(|v| v.as_bool()).unwrap_or(false);

        // Run the database write on a background thread
        std::thread::spawn(move || {
            let mut write_opts = WriteOptions::default();
            write_opts.set_sync(sync);
            let result = db.put_opt(&key_bytes, &value_bytes, &write_opts)
                .map_err(|e| napi::Error::from_reason(e.to_string()));
            callback.call(result, |&mut _| Ok(()));
        });
    }

    #[napi(ts_args_type = "key: string | Buffer, options: any, callback: (err: Error | null, value: Buffer | null) => void")]
    pub fn get(&self, key: napi::Either<String, napi::Buffer>, _options: serde_json::Value, callback: ThreadsafeFunction<Option<napi::Buffer>, ErrorStrategy::CalleeHandled>) {
        let db = self.db.clone();
        let key_bytes = either_to_vec(key);
        
        // Run the database read on a background thread
        std::thread::spawn(move || {
            match db.get(&key_bytes) {
                Ok(Some(value)) => callback.call(Ok(Some(value.into())), |&mut _| Ok(())),
                Ok(None) => callback.call(Ok(None), |&mut _| Ok(())),
                Err(e) => callback.call(Err(napi::Error::from_reason(e.to_string())), |&mut _| Ok(())),
            }
        });
    }

    #[napi(ts_args_type = "key: string | Buffer, options: any, callback: (err: Error | null) => void")]
    pub fn del(&self, key: napi::Either<String, napi::Buffer>, options: serde_json::Value, callback: ThreadsafeFunction<(), ErrorStrategy::CalleeHandled>) {
        let db = self.db.clone();
        let key_bytes = either_to_vec(key);
        let sync = options.get("sync").and_then(|v| v.as_bool()).unwrap_or(false);
        
        // Run the database delete on a background thread
        std::thread::spawn(move || {
            let mut write_opts = WriteOptions::default();
            write_opts.set_sync(sync);
            let result = db.delete_opt(&key_bytes, &write_opts)
                .map_err(|e| napi::Error::from_reason(e.to_string()));
            callback.call(result, |&mut _| Ok(()));
        });
    }
}

// Helper function to convert napi's Either type into a byte vector
fn either_to_vec(e: napi::Either<String, napi::Buffer>) -> Vec<u8> {
    match e {
        napi::Either::A(s) => s.into_bytes(),
        napi::Either::B(b) => b.into(),
    }
}