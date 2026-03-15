const os = require('os');

const platform = os.platform();
const arch = os.arch();

let binding;
try {
    if (platform === 'darwin' && arch === 'arm64') {
        binding = require('./npm/darwin-arm64/rocksdb-piper-node.darwin-arm64.node');
    } else if (platform === 'linux' && arch === 'x64') {
        binding = require('./npm/linux-x64-gnu/rocksdb-piper-node.linux-x64-gnu.node');
    } else {
        throw new Error(`Unsupported platform: ${platform}-${arch}`);
    }
} catch (e) {
    throw new Error(`Failed to load native binding: ${e.message}`);
}

module.exports = binding;
