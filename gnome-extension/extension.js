import Atk from 'gi://Atk';
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

import {GROUPS, formatGroups, parseFrame, valueMarkup} from './panel-format.js';

const SECONDARY_METRICS = new Set([
    'CPU temp', 'GPU temp', 'VRAM', 'Swap', 'Network up', 'Storage read', 'Storage write', 'Inodes',
]);

export default class SysMonExtension extends Extension {
    enable() {
        this._enabled = true;
        this._indicator = new PanelMenu.Button(0.0, 'sys-mon', true);
        this._indicator.setMenu(null);
        this._indicator.setSensitive(false);
        this._indicator.accessible_role = Atk.Role.PANEL;
        this._indicator.add_style_class_name('sys-mon-panel-button');
        this._strip = new St.BoxLayout({style_class: 'sys-mon-strip'});
        this._indicator.add_child(this._strip);
        this._groups = GROUPS.map(group => {
            const actor = new St.BoxLayout({
                style_class: 'sys-mon-group', reactive: false, track_hover: false,
                accessible_name: `${group.name}: waiting for readings`,
            });
            actor.add_child(new St.Icon({
                gicon: new Gio.FileIcon({
                    file: this.dir.get_child('icons').get_child(`${group.icon}-symbolic.svg`),
                }),
                style_class: 'system-status-icon sys-mon-glyph', icon_size: 16,
                y_align: Clutter.ActorAlign.CENTER,
            }));
            const values = group.labels.map((label, index) => {
                if (index > 0) {
                    actor.add_child(new St.Label({
                        text: '/', style_class: 'sys-mon-separator',
                        y_align: Clutter.ActorAlign.CENTER,
                    }));
                }
                const value = new St.Label({
                    text: '—',
                    style_class: SECONDARY_METRICS.has(label)
                        ? 'sys-mon-values sys-mon-secondary'
                        : 'sys-mon-values',
                    // Keep each reading centered on a fixed anchor. Right
                    // alignment puts all spare space after the preceding slash.
                    x_align: Clutter.ActorAlign.CENTER, y_align: Clutter.ActorAlign.CENTER,
                });
                if (label.startsWith('Network ') || label.startsWith('Storage '))
                    value.add_style_class_name('sys-mon-rate');
                const slot = new St.Bin({
                    style: `width: ${group.widths[index]}em;`,
                    clip_to_allocation: true,
                });
                slot.set_child(value);
                actor.add_child(slot);
                return value;
            });
            const state = {actor, values};
            this._strip.add_child(actor);
            return state;
        });
        Main.panel.addToStatusArea(this.uuid, this._indicator, -1, 'left');
        this._setUnavailable('Waiting for readings');
        this._startBackend();
        this._watchdog = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 3, () => {
            if (this._process && GLib.get_monotonic_time() - this._lastFrame > 10_000_000)
                this._backendFailed('Readings timed out');
            return GLib.SOURCE_CONTINUE;
        });
    }

    _startBackend() {
        if (!this._enabled)
            return;
        try {
            const executable = GLib.build_filenamev([this.path, 'sys-mon']);
            this._cancellable = new Gio.Cancellable();
            this._process = Gio.Subprocess.new([executable, '--panel-stream'],
                Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE |
                Gio.SubprocessFlags.STDERR_SILENCE);
            this._input = new Gio.DataInputStream({
                base_stream: this._process.get_stdout_pipe(), close_base_stream: true,
            });
            this._lastFrame = GLib.get_monotonic_time();
            this._readNext(this._input, this._cancellable);
        } catch (error) {
            console.error(`sys-mon: ${error.message}`);
            this._backendFailed('Unable to start sys-mon');
        }
    }

    _readNext(input, cancellable) {
        input.read_line_async(GLib.PRIORITY_LOW, cancellable, (stream, result) => {
            try {
                const [line] = stream.read_line_finish_utf8(result);
                if (!this._enabled || cancellable.is_cancelled())
                    return;
                if (line === null)
                    throw new Error('Telemetry process stopped');
                const cells = parseFrame(line);
                this._lastFrame = GLib.get_monotonic_time();
                this._update(cells);
                this._readNext(input, cancellable);
            } catch (error) {
                if (!this._enabled || cancellable.is_cancelled())
                    return;
                console.error(`sys-mon: ${error.message}`);
                this._backendFailed('Readings unavailable; retrying');
            } finally {
                if (cancellable.is_cancelled())
                    stream.close(null);
            }
        });
    }

    _update(cells) {
        const groups = formatGroups(cells);
        for (const [index, group] of groups.entries()) {
            const state = this._groups[index];
            for (const [slot, value] of state.values.entries())
                value.clutter_text.set_markup(valueMarkup(group.values[slot]));
            state.actor.accessible_name = group.description;
        }
    }

    _setUnavailable(reason) {
        for (const [index, state] of this._groups.entries()) {
            for (const value of state.values)
                value.text = '—';
            state.actor.accessible_name = `${GROUPS[index].name}: ${reason}`;
        }
    }

    _stopBackend() {
        this._cancellable?.cancel();
        this._cancellable = null;
        // Closing stdin lets Rust stop its workers and reap helper processes.
        this._process?.get_stdin_pipe().close(null);
        // A full stdout pipe can block Rust before it observes stdin EOF.
        // When a read is pending, its cancellation callback closes this stream.
        try {
            this._input?.close(null);
        } catch (error) {
            if (!error.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.PENDING))
                console.error(`sys-mon: ${error.message}`);
        }
        this._process = null;
        this._input = null;
    }

    _backendFailed(reason) {
        this._stopBackend();
        this._setUnavailable(reason);
        if (!this._retry && this._enabled) {
            this._retry = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 5, () => {
                this._retry = 0;
                this._startBackend();
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    disable() {
        this._enabled = false;
        for (const source of [this._watchdog, this._retry]) {
            if (source)
                GLib.Source.remove(source);
        }
        this._watchdog = this._retry = 0;
        this._stopBackend();
        this._indicator?.destroy();
        this._indicator = null;
        this._groups = null;
    }
}
