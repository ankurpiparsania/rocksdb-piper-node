export interface RocksDbOptions {
    compression_type?: string;
    block_cache_size_mb?: number;
    write_buffer_size_mb?: number;
    max_write_buffer_number?: number;
    wal_size_limit_mb?: number;
    wal_ttl_seconds?: number;
}

export interface WriteOptions {
    sync?: boolean;
}

export interface ReadOptions {
    // Reserved for parity with callback-style RocksDB APIs.
    [key: string]: unknown;
}

export class RocksDB {
    constructor(path: string, options?: RocksDbOptions);

    open(options: ReadOptions, callback: (err: Error | null) => void): void;

    put(
        key: string | Buffer,
        value: string | Buffer,
        options: WriteOptions,
        callback: (err: Error | null) => void
    ): void;

    get(
        key: string | Buffer,
        options: ReadOptions,
        callback: (err: Error | null, value: Buffer | null) => void
    ): void;

    del(
        key: string | Buffer,
        options: WriteOptions,
        callback: (err: Error | null) => void
    ): void;
}
