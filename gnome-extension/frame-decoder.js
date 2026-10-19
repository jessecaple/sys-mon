// Bound unfinished lines before decoding JSON inside the Shell process.
export const MAX_FRAME_BYTES = 65_536;

export class FrameDecoder {
    constructor() {
        this._buffer = new Uint8Array(MAX_FRAME_BYTES);
        this._length = 0;
        this._decoder = new TextDecoder('utf-8', {fatal: true});
    }

    push(bytes) {
        const lines = [];
        let start = 0;
        for (let index = 0; index <= bytes.length; index++) {
            if (index < bytes.length && bytes[index] !== 10)
                continue;
            const count = index - start;
            if (count > MAX_FRAME_BYTES - this._length)
                throw new Error('Oversized sys-mon telemetry frame');
            this._buffer.set(bytes.subarray(start, index), this._length);
            this._length += count;
            if (index < bytes.length) {
                lines.push(this._decoder.decode(this._buffer.subarray(0, this._length)));
                this._length = 0;
            }
            start = index + 1;
        }
        return lines;
    }
}
