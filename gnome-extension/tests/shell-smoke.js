// Run with gnome-shell-test-tool --headless --extension <bundle> <this file>.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

export const METRICS = {};
const UUID = 'sys-mon@jcaple.dev';
const wait = ms => new Promise(resolve => GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
    resolve();
    return GLib.SOURCE_REMOVE;
}));
const check = (condition, message) => { if (!condition) throw new Error(message); };

export async function run() {
    await wait(4500);
    const extension = Main.extensionManager.lookup(UUID);
    check(extension?.stateObj?._groups?.length === 6, 'Expected six panel groups');
    let state = extension.stateObj;
    for (const group of state._groups) {
        const icon = group.actor.get_first_child();
        check(icon instanceof St.Icon, 'Groups must use symbolic icons');
        check(icon.gicon.get_file().query_exists(null), 'Bundled icon must exist');
        check(icon.icon_size === 16, 'Icons must have consistent 16px sizing');
        check(icon.width === state._groups[0].actor.get_first_child().width, 'Icon allocations must match');
    }
    check(state._groups[0].values[0].text.includes('%'), 'CPU must have a real reading');
    check(state._groups[2].values.length === 2 && state._groups[2].values.every(value => value.text.endsWith('%')),
        'RAM and swap must show percentages');
    check(state._groups[3].values[0].text.endsWith('%'), 'Storage remaining must show a percentage');
    check(state._groups[0].values[1].text.endsWith('°'), 'CPU temperature must remain Celsius');
    check(state._groups[1].values[1].text.endsWith('%'), 'VRAM must follow GPU usage');
    check(state._groups[1].values[2].text.endsWith('°'), 'GPU temperature must come last and remain Celsius');
    check(state._groups[4].actor.accessible_name.includes('/tmp inodes:'), 'Inodes must refer to /tmp');
    check(state._groups[4].actor.accessible_name.includes('/tmp filesystem:'), 'Capacity must include /tmp');
    for (const group of state._groups) {
        for (const value of group.values) {
            if (value.text !== '—')
                check(value.get_preferred_width(-1)[1] <= value.get_parent().width,
                    'Padded readings must fit their compact slots');
        }
    }
    // Compare allocations after forcing digit/unit changes and unavailable
    // states. Each slot and every later group must keep the same geometry.
    const geometry = () => state._groups.map(group => ({
        x: group.actor.get_transformed_position()[0],
        width: group.actor.width,
        separators: group.actor.get_children()
            .filter(actor => actor.has_style_class_name('sys-mon-separator'))
            .map(actor => ({x: actor.get_transformed_position()[0], width: actor.width})),
        slots: group.values.map(value => ({
            x: value.get_parent().get_transformed_position()[0],
            width: value.get_parent().width,
        })),
    }));
    const checkCenters = () => {
        for (const group of state._groups) {
            for (const value of group.values) {
                const slot = value.get_parent();
                const anchor = slot.get_transformed_position()[0] + slot.width / 2;
                const center = value.get_transformed_position()[0] + value.width / 2;
                // Odd/even text widths round to the nearest device pixel.
                check(Math.abs(center - anchor) <= 0.5, 'Readings must stay centered on fixed anchors');
            }
        }
    };
    checkCenters();
    const before = JSON.stringify(geometry());
    const oldValues = state._groups.map(group => group.values.map(value => value.text));
    for (const group of state._groups)
        for (const value of group.values)
            value.text = '999M';
    await wait(100);
    check(JSON.stringify(geometry()) === before, 'Extra digits must not move slots, separators or groups');
    checkCenters();
    for (const index of [3, 5]) {
        for (const value of state._groups[index].values.filter(value => value.has_style_class_name('sys-mon-rate')))
            check(value.get_preferred_width(-1)[1] <= value.get_parent().width, `Three-digit speeds must fit: ${value.get_preferred_width(-1)[1]} > ${value.get_parent().width}`);
    }
    for (const group of state._groups)
        for (const value of group.values)
            value.text = '—';
    await wait(100);
    check(JSON.stringify(geometry()) === before, 'Unavailable values must not move slots or groups');
    checkCenters();
    for (const [index, group] of state._groups.entries())
        for (const [slot, value] of group.values.entries())
            value.text = oldValues[index][slot];
    check(state._indicator.menu === null, 'Display-only strip must not have a click menu');
    check(!state._indicator.reactive && !state._indicator.can_focus && !state._indicator.track_hover,
        'Display-only strip must not capture clicks, focus or hover');
    check(state._groups.every(group => !group.actor.reactive && !group.actor.track_hover),
        'Groups must not respond to hover');

    state._process.force_exit();
    await wait(1000);
    check(state._groups[0].values[0].text === '—', 'Backend exit must clear stale readings');
    await wait(9000);
    check(state._groups[0].values[0].text.includes('%'), 'Backend must restart and recover');
    const pid = state._process.get_identifier();
    Main.extensionManager.disableExtension(UUID);
    await wait(2000);
    check(!Main.panel.statusArea[UUID], 'Disable must remove the panel actor');
    check(!Gio.File.new_for_path(`/proc/${pid}`).query_exists(null), 'Disable must stop the backend');
    check(state._watchdog === 0 && state._retry === 0, 'Disable must remove timers');
    Main.extensionManager.enableExtension(UUID);
    await wait(4500);
    state = Main.extensionManager.lookup(UUID).stateObj;
    check(state._groups[0].values[0].text.includes('%'), 'Re-enable must collect readings');
    Main.extensionManager.disableExtension(UUID);
    await wait(1500);
    print('SYS_MON_SMOKE_OK: readings, fixed slot geometry, display-only behavior, recovery, disable and re-enable');
}
