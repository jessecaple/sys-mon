import assert from 'node:assert/strict';
import test from 'node:test';
import {FrameDecoder, MAX_FRAME_BYTES} from '../frame-decoder.js';

const encode = text => new TextEncoder().encode(text);

test('decodes split UTF-8 and multiple frames with an unfinished tail', () => {
    const decoder = new FrameDecoder();
    const bytes = encode('°\nfirst\nlast');
    assert.deepEqual(decoder.push(bytes.subarray(0, 1)), []);
    assert.deepEqual(decoder.push(bytes.subarray(1)), ['°', 'first']);
    assert.deepEqual(decoder.push(encode('\n')), ['last']);
});

test('caps an unfinished frame across fixed-size async reads', () => {
    const decoder = new FrameDecoder();
    const chunk = new Uint8Array(4096).fill(120);
    for (let i = 0; i < MAX_FRAME_BYTES / chunk.length; i++)
        assert.deepEqual(decoder.push(chunk), []);
    assert.throws(() => decoder.push(encode('x')), /Oversized/);
});

test('the frame bound counts UTF-8 bytes, and resets for each completed frame', () => {
    const decoder = new FrameDecoder();
    const frame = '°'.repeat(MAX_FRAME_BYTES / 2);
    for (let i = 0; i < 3; i++)
        assert.deepEqual(decoder.push(encode(`${frame}\n`)), [frame]);
    assert.throws(() => decoder.push(encode(`${frame}x\n`)), /Oversized/);
});

test('invalid UTF-8 fails instead of publishing a damaged frame', () => {
    assert.throws(() => new FrameDecoder().push(new Uint8Array([255, 10])));
});
