# MonKey

Open-source macOS/cross-platform tooling for the **Monka 3075 Pro** mechanical keyboard —
and, eventually, a physical AI coding-agent status display on its built-in LCD.

> ⚠️ **Early research stage.** No usable driver yet. This repo currently documents
> verified hardware findings so other owners of this board don't have to start from zero.

---

## Why

The Monka 3075 Pro ships with a Windows-only vendor driver. There is no macOS support,
no documented protocol, and no way to script the keyboard's LCD or per-key RGB.

The longer-term goal is to turn the keyboard into an ambient status surface for AI coding
agents (Claude Code, and others): glanceable state on the LCD and RGB — especially
*"the agent is blocked waiting on you"* — without having to watch a terminal.

## Hardware target

| | |
| :--- | :--- |
| Model | Monka 3075 Pro (75%, 81 keys) |
| VID / PID | `0x05AC` / `0x024F` (Apple VID is spoofed by the OEM) |
| OEM solution | Shenzhen HFD Technology — `RKGK890` |
| Display | 128×128 RGB565 TFT |

The same `05AC:024F` + `RKGK890` triple appears on other HFD-based boards, so findings here
should transfer to that family.

## Verified findings

Measured on real hardware over USB (macOS + Chrome WebHID). The device enumerates as
**two separate HID entries**:

### Entry A — bulk / display pipe ✅ openable from the browser

```
Usage Page 0xFF68 / Usage 0x61   (vendor collection, alone on its interface)
   OUT report ID 0 = 4096 bytes
   IN  report ID 0 =   64 bytes
```

Because the vendor collection does **not** share an interface with any protected collection,
Chrome can open it and macOS does not require Input Monitoring.

`128 × 128 × 2 = 32768` bytes per frame ÷ 4096 = **8 chunks per frame**.

### Entry B — configuration interface ❌ write-blocked in the browser

```
Usage Page 0x0C   / Usage 0x01   (Consumer Control — protected)
Usage Page 0x01   / Usage 0x02   (Mouse           — protected)
Usage Page 0xFFFF / Usage 0x01   IN report ID 5 = 3 bytes
```

The 64-byte **feature** reports the vendor driver uses for RGB, keymap and macro writes are
stripped by Chrome, because this interface is co-resident with protected collections.
Opening it at all requires granting Chrome **Input Monitoring** on macOS.

### Consequence

| Feature | Interface | WebHID | Native (hidapi / IOKit) |
| :--- | :--- | :---: | :---: |
| LCD / display | `0xFF68`, OUT 4096 | ✅ | ✅ |
| Per-key RGB | `0xFFFF`, feature 64 | ❌ | ✅ |
| Keymap / macros | `0xFFFF`, feature 64 | ❌ | ✅ |

A complete driver therefore needs a **native host** (Tauri + `hidapi`, or IOKit).
WebHID remains useful for the display path and for quick experimentation.

## Roadmap

- [ ] Agent state daemon — normalize Claude Code hook events into a small state machine
      (`IDLE / THINKING / EDITING / RUNNING / WAITING_FOR_YOU / ERROR / DONE`), renderer-agnostic
- [ ] Decode the `0xFF68` bulk chunk header (USB capture against the vendor driver)
- [ ] Native transport (Tauri v2 + `hidapi`) for the configuration interface
- [ ] Ambient RGB driven by agent state
- [ ] LCD status / animation driven by agent state

## Prior art

This work stands on public reverse-engineering of the same or adjacent OEM families:

- [`rcsn01/GMK-67-Driver`](https://github.com/rcsn01/GMK-67-Driver) — **same `05AC:024F` / `RKGK890`**;
  decoded the `04 xx` configuration command family and shipped a working macOS implementation.
- [`wsclx/ak820pro-modder`](https://github.com/wsclx/ak820pro-modder) — Ajazz/Sonix family;
  documented TFT animation upload and the wider command table.
- [`Aiacos/ajazz-control-center`](https://github.com/Aiacos/ajazz-control-center) — TFT and RTC probes.

## Safety

Reverse-engineering USB HID devices can brick them permanently.

- Never send guessed opcodes. Bootloader/DFU commands share the same command space.
- Capture first (passively), replay second.
- Read commands before write commands; RAM writes before flash writes.
- Keep the board wired and charged before any flash write — a brown-out mid-write can
  corrupt a sector and brick the keyboard.

## Status & contributions

Nothing here is stable. If you own a board in this family, interface dumps and USB captures
are the most useful thing you can contribute.

## License

MIT
