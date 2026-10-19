// Keep panel grouping independent of Shell actors and telemetry collection.
// Slot widths are in theme font units, independent of the current reading.
export const GROUPS = [
    {icon: 'processor', name: 'CPU', labels: ['CPU', 'CPU temp'], widths: [1.9, 1.9]},
    {icon: 'graphics-card', name: 'GPU', labels: ['GPU', 'VRAM', 'GPU temp'], widths: [1.9, 1.9, 1.9]},
    {icon: 'ram', name: 'RAM', labels: ['RAM', 'Swap'], widths: [1.9, 1.9]},
    {icon: 'disk', name: 'Storage', labels: ['Storage', 'Storage read', 'Storage write'], widths: [1.9, 2.5, 2.5]},
    {icon: 'folder-temporary', name: '/tmp', labels: ['/tmp', 'Inodes'], widths: [1.9, 1.9]},
    {icon: 'data-transfer', name: 'Network', labels: ['Network down', 'Network up'], widths: [2.5, 2.5]},
];

const FRAME_LABELS = GROUPS.flatMap(group => group.labels);

// Escape telemetry text before handing it to Pango; only padding gets markup.
export function valueMarkup(value) {
    const escape = text => text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    const padding = value.match(/^([<-]?)(0+)(?=\d)/);
    if (!padding)
        return escape(value);
    return `${escape(padding[1])}<span alpha="50%">${padding[2]}</span>${escape(value.slice(padding[0].length))}`;
}

const MISSING = new Set([
    'Waiting', 'Unavailable', 'No access', 'Invalid reading',
    'Source error', 'Timed out',
]);

// Promote the suffix before rounding would need a fourth digit.
function wholeAmount(number, unit, units, base) {
    while (Math.round(number) > 999 && unit < units.length - 1) {
        number /= base;
        unit++;
    }
    return `${String(Math.round(number)).padStart(3, '0')}${units[unit]}`;
}

export function compact(value, label = '') {
    if (MISSING.has(value))
        return '—';
    const percent = value.match(/^(<)?(\d+(?:\.\d+)?)%$/);
    if (percent) {
        const number = Number(percent[2]);
        if (percent[1] || (number > 0 && number < 1))
            return '<1%';
        return displayPercent(number);
    }
    const temperature = value.match(/^(-?\d+(?:\.\d+)?)\s*°C$/);
    if (temperature) {
        const number = Math.round(Number(temperature[1]));
        if (number > 99)
            return '99+°';
        if (number < -99)
            return '<-99°';
        return `${number < 0 ? '-' : ''}${String(Math.abs(number)).padStart(2, '0')}°`;
    }
    if (label === 'Inodes') {
        const count = value.match(/^(\d+(?:\.\d+)?)([KM])?$/);
        if (count) {
            const units = ['', 'K', 'M', 'G', 'T', 'P', 'E'];
            return wholeAmount(Number(count[1]), units.indexOf(count[2] ?? ''), units, 1000);
        }
    }
    return value.replace(/(<)?(\d+(?:\.\d+)?)\s*(KiB|MiB|GiB|TiB|B)(?:\/s)?/g,
        (_, less, number, unit) => {
            const units = ['B', 'K', 'M', 'G', 'T', 'P', 'E'];
            const index = ['B', 'KiB', 'MiB', 'GiB', 'TiB'].indexOf(unit);
            if (less)
                return `<${String(Math.max(1, Math.ceil(Number(number)))).padStart(2, '0')}${units[index]}`;
            return wholeAmount(Number(number), index, units, 1024);
        });
}

const PERCENT_METRICS = new Set(['RAM', 'Swap', 'VRAM', 'Storage', '/tmp', 'Inodes']);

function displayPercent(number) {
    if (number > 0 && number < 1)
        return '<1%';
    return `${String(Math.min(99, Math.round(number))).padStart(2, '0')}%`;
}

function metricPercentage(metric, index) {
    if (MISSING.has(metric.lines[index].value))
        return null;
    if (metric.label === 'Storage')
        return metric.percentages?.[index] ?? null;
    const fraction = metric.lines[index].rail;
    return Number.isFinite(fraction) ? fraction * 100 : null;
}

export function parseFrame(line) {
    const frame = JSON.parse(line);
    if (frame.version !== 1 || !Array.isArray(frame.cells) || frame.cells.length !== 14)
        throw new Error('Unsupported sys-mon telemetry frame');
    const labels = new Set();
    for (const cell of frame.cells) {
        if (!cell || typeof cell.label !== 'string' || labels.has(cell.label) ||
            !Array.isArray(cell.lines) || cell.lines.length < 1 || cell.lines.length > 2)
            throw new Error('Invalid sys-mon metric');
        if (cell.amount !== undefined && (!cell.amount ||
            !Number.isSafeInteger(cell.amount.used_bytes) || cell.amount.used_bytes < 0 ||
            !Number.isSafeInteger(cell.amount.total_bytes) || cell.amount.total_bytes < 0))
            throw new Error('Invalid sys-mon capacity');
        if (cell.percentages !== undefined && (!Array.isArray(cell.percentages) ||
            cell.percentages.length !== cell.lines.length || cell.percentages.some(value =>
                !Number.isFinite(value) || value < 0 || value > 100)))
            throw new Error('Invalid sys-mon percentages');
        labels.add(cell.label);
        for (const value of cell.lines) {
            if (typeof value.value !== 'string' || typeof value.description !== 'string' ||
                value.value.length > 128 || value.description.length > 1024)
                throw new Error('Invalid sys-mon value');
            if (value.rail != null && (!Number.isFinite(value.rail) || value.rail < 0 || value.rail > 1))
                throw new Error('Invalid sys-mon fraction');
        }
    }
    if (FRAME_LABELS.some(label => !labels.has(label)))
        throw new Error('Incomplete sys-mon telemetry frame');
    return frame.cells;
}

export function formatGroups(cells) {
    const byLabel = new Map(cells.map(cell => [cell.label, cell]));
    return GROUPS.map(group => {
        const metrics = group.labels.map(label => byLabel.get(label));
        return {
            ...group,
            values: metrics.map(metric => metric.lines.map((line, index) => {
                if (!PERCENT_METRICS.has(metric.label))
                    return compact(line.value, metric.label);
                const percentage = metricPercentage(metric, index);
                if (percentage === null)
                    return '—';
                const path = metric.label === 'Storage' ? line.value.match(/^(\/\S*)\s/)?.[1] : null;
                return `${path ? `${path} ` : ''}${displayPercent(percentage)}`;
            }).join(' · ')),
            description: metrics.flatMap(metric => metric.lines.map((line, index) => {
                if (!PERCENT_METRICS.has(metric.label))
                    return line.description;
                const percentage = metricPercentage(metric, index);
                const meaning = metric.label === 'Storage' ? 'remaining' : 'used';
                return percentage === null ? line.description
                    : `${line.description}; ${Number(percentage.toFixed(2))} percent ${meaning}`;
            })).join('\n'),
            metrics,
        };
    });
}
