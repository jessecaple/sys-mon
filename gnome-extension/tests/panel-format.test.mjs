import assert from 'node:assert/strict';
import test from 'node:test';
import {GROUPS, compact, formatGroups, parseFrame, valueMarkup} from '../panel-format.js';

const frame = () => ({version: 1, cells: GROUPS.flatMap(group => group.labels).map(label => ({
    label, lines: [{value: 'Waiting', description: `${label}: waiting`}],
}))});

test('dims only padding zeros, preserves significant zeros, and escapes markup', () => {
    for (const [value, markup] of [
        ['03%', '<span alpha="50%">0</span>3%'],
        ['00%', '<span alpha="50%">0</span>0%'],
        ['000B', '<span alpha="50%">00</span>0B'],
        ['009K', '<span alpha="50%">00</span>9K'],
        ['010M', '<span alpha="50%">0</span>10M'],
        ['30%', '30%'], ['100K', '100K'], ['—', '—'],
        ['<1%', '&lt;1%'], ['<01B', '&lt;<span alpha="50%">0</span>1B'],
        ['-03°', '-<span alpha="50%">0</span>3°'],
        ['<b>&test</b>', '&lt;b&gt;&amp;test&lt;/b&gt;'],
    ]) assert.equal(valueMarkup(value), markup);
});

test('keeps readable compact units, tiny values, and unavailable states distinct', () => {
    for (const [input, output] of [
        ['2.0 GiB', '002G'], ['12.3 MiB/s', '012M'], ['61 °C', '61°'],
        ['0 B/s', '000B'], ['<1 B/s', '<01B'], ['<0.1%', '<1%'],
        ['Unavailable', '—'], ['Timed out', '—'],
    ]) assert.equal(compact(input), output);
});

test('uses whole numbers, promotes four-digit amounts, and caps displayed percentages', () => {
    for (const [input, label, output] of [
        ['4.8 GiB', 'VRAM', '005G'], ['26.4 GiB', 'RAM', '026G'],
        ['999 KiB/s', 'Network down', '999K'], ['1000 KiB/s', 'Network down', '001M'],
        ['999.5 MiB/s', 'Storage write', '001G'], ['1000 B/s', 'Storage read', '001K'],
        ['2.5K', 'Inodes', '003K'], ['999.9K', 'Inodes', '001M'],
        ['1000.0M', 'Inodes', '001G'], ['0', 'Inodes', '000'],
        ['100%', 'CPU', '99%'], ['99.9%', 'GPU', '99%'],
        ['0.5%', 'GPU', '<1%'], ['0%', 'CPU', '00%'],
        ['3%', 'CPU', '03%'], ['9.4%', 'GPU', '09%'],
        ['0 °C', 'CPU temp', '00°'], ['7 °C', 'GPU temp', '07°'],
        ['-3 °C', 'CPU temp', '-03°'], ['9 KiB/s', 'Network up', '009K'],
        ['99 °C', 'CPU temp', '99°'], ['100 °C', 'CPU temp', '99+°'],
    ]) assert.equal(compact(input, label), output);
});

test('groups all metrics once and preserves units, paths, and descriptions', () => {
    const source = frame();
    source.cells.find(cell => cell.label === 'CPU').lines = [{value: '25%', description: 'CPU: 25 percent'}];
    source.cells.find(cell => cell.label === 'CPU temp').lines = [{value: '60 °C', description: 'CPU: 60 degrees'}];
    source.cells.find(cell => cell.label === 'Storage').lines = [
        {value: '/ 200 GiB', description: 'Storage /: 200 GiB remaining'},
        {value: '/home 50.0 GiB', description: 'Storage /home: 50 GiB remaining'},
    ];
    source.cells.find(cell => cell.label === 'Storage').percentages = [20, 25];
    const groups = formatGroups(parseFrame(JSON.stringify(source)));
    assert.equal(groups.length, 6);
    assert.deepEqual(groups[0].values, ['25%', '60°']);
    assert.equal(groups[0].description, 'CPU: 25 percent\nCPU: 60 degrees');
    assert.deepEqual(groups[3].values, ['/ 20% · /home 25%', '—', '—']);
    assert.deepEqual(groups.map(group => group.labels), [
        ['CPU', 'CPU temp'], ['GPU', 'VRAM', 'GPU temp'], ['RAM', 'Swap'],
        ['Storage', 'Storage read', 'Storage write'], ['/tmp', 'Inodes'],
        ['Network down', 'Network up'],
    ]);
    assert.equal(groups.flatMap(group => group.metrics).length, 14);
});

test('shows capped resources as percentages, preserves temperatures, and keeps zero and unavailable distinct', () => {
    const source = frame();
    for (const [label, rail] of [['RAM', 0.5], ['Swap', 0], ['VRAM', 0.03], ['/tmp', 0.125], ['Inodes', 1]]) {
        const metric = source.cells.find(cell => cell.label === label);
        metric.lines = [{value: '123 MiB', description: label, rail}];
    }
    const temperature = source.cells.find(cell => cell.label === 'GPU temp');
    temperature.lines = [{value: '40 °C', description: 'GPU temperature', rail: 0.5}];
    const storage = source.cells.find(cell => cell.label === 'Storage');
    storage.lines = [{value: '20 GiB', description: 'Storage: 20 GiB remaining', rail: 0.7}];
    storage.percentages = [20];
    const groups = formatGroups(parseFrame(JSON.stringify(source)));
    assert.deepEqual(groups[2].values, ['50%', '00%']);
    assert.deepEqual(groups[1].values, ['—', '03%', '40°']);
    assert.deepEqual(groups[4].values, ['13%', '99%']);
    assert.equal(groups[3].values[0], '20%');
    assert.ok(groups[3].description.includes('20 percent remaining'));
    assert.ok(groups[4].description.includes('100 percent used'));
    const ram = source.cells.find(cell => cell.label === 'RAM');
    ram.lines[0].rail = null;
    assert.equal(formatGroups(source.cells)[2].values[0], '—');
    assert.deepEqual(formatGroups(source.cells)[2].widths, groups[2].widths);
    ram.lines[0].rail = 0.001;
    assert.equal(formatGroups(source.cells)[2].values[0], '<1%');
});

test('rejects protocol changes, incomplete frames, duplicate labels, and malformed values', () => {
    for (const mutate of [
        source => source.version = 2,
        source => source.cells[0].percentages = [101],
        source => source.cells[0].percentages = [],
        source => source.cells[0].lines[0].rail = -1,
        source => source.cells[0].amount = {used_bytes: -1, total_bytes: 1024},
        source => source.cells[0].amount = {used_bytes: 0, total_bytes: 'unknown'},
        source => source.cells.pop(),
        source => source.cells[0].label = 'Wrong metric',
        source => source.cells[0] = source.cells[1],
        source => source.cells[0].lines = [],
        source => source.cells[0].lines[0].value = 42,
        source => source.cells[0].lines[0].description = 'x'.repeat(1025),
    ]) {
        const source = frame();
        mutate(source);
        assert.throws(() => parseFrame(JSON.stringify(source)));
    }
});
