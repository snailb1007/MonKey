# Pitfalls Research

**Domain:** Custom Keyboard Drivers, USB HID Reverse Engineering, Embedded LCD Streaming & macOS System Security
**Researched:** 2026-09-13
**Confidence:** HIGH (Derived from physical hardware measurements, verified prior art on identical `05AC:024F` / `RKGK890` hardware, and extracted vendor driver binaries)

---

## Critical Pitfalls

### Pitfall 1: Blind Opcodes & Accidental Bootloader/DFU Invocation (Permanent Hardware Bricking)

**What goes wrong:**
Sending speculative, fuzzed, or guessed byte sequences to the vendor HID interface can trigger the MCU's internal bootloader (e.g., Sonix/HFD ISP bootloader mode with PID `0x7140`), trigger a flash mass-erase command, or overwrite the boot vector table. The keyboard immediately disconnects, stops responding to keypresses and USB enumeration, and becomes permanently bricked. Recovery requires desoldering the casing and connecting hardware SWD/JTAG debug probes or factory flash programmers.

**Why it happens:**
Developers coming from web/desktop environments assume that sending an invalid opcode returns an error code (like `EINVAL` or `404`). However, low-cost keyboard microcontrollers (such as the Shenzhen HFD / Sonix SN32F248B Cortex-M0 or 8051 variants) run bare-metal firmware without memory protection units (MPU) or command validation layers. The vendor command dispatcher (`0x04 xx`, `0xA1`, `0xAA`) shares memory, jump tables, and direct SPI flash controller registers with the ISP bootloader.

**How to avoid:**
1. **Zero Speculative Writes:** Strictly prohibit random probing or fuzzing on write endpoints.
2. **Schema-Driven Opcode Whitelisting:** Implement a strict capability matrix in `monkey-core`. Only dispatch opcodes verified through differential USB packet captures (`04 18` START, `04 13` Apply RGB, `04 20` RGB table, `04 02` SAVE, `04 F0` END). Reject all unmapped opcodes at the driver layer before they touch the USB transport.
3. **Bootloader Quarantine:** Any opcode pattern matching known ISP triggers (e.g., jump-to-ISP, mass erase, flash boot sector unlock) must be placed on an immutable denylist that cannot be called even with manual CLI flags.
4. **Passive Capture First:** Follow the rule: *Passive Capture $\rightarrow$ Differential Analysis $\rightarrow$ Read Probing $\rightarrow$ Verified Write Replay*. Never invert this order.

**Warning signs:**
- Device suddenly re-enumerates with a different USB VID/PID (e.g., `0x0C45:0x7140` or generic `0xFFFF:0xFFFF`).
- Device descriptor read fails during USB enumeration.
- All LEDs shut off immediately after sending a packet and do not light up on power cycle.

**Phase to address:**
Phase 2: Protocol Safety Rails & Schema-Driven Matrix

---

### Pitfall 2: High-Frequency Flash Commit Exhaustion & Low-Battery Brownout

**What goes wrong:**
1. **Flash Wear-Out:** Real-time ambient RGB lighting (such as pulsating to Claude Code agent state or dragging a color picker) writes directly to on-board SPI NOR flash instead of volatile SRAM. The NOR Flash cell endurance is typically rated for only 10,000 to 100,000 erase/write cycles. Streaming writes at 10–30 Hz exhausts this endurance within hours or days, causing permanent sector degradation, corrupted settings, or boot failure.
2. **Low-Battery Brownout:** Initiating a flash sector erase/write cycle while operating on battery power with low voltage (<20%) triggers a high-current charge pump pulse. The resulting voltage sag causes an MCU brownout reset mid-write, corrupting partition tables, macro allocations, or active profiles.

**Why it happens:**
In the OEM protocol, certain commands (such as the `04 02` commit command or standalone mode commands) trigger immediate NOR flash sector writes (`Sector Erase` + `Page Program`). Developers unfamiliar with the hardware lifecycle treat LED color updates like CSS properties, emitting writes on every animation tick or mouse movement.

**How to avoid:**
1. **Two-Tier State Sync Architecture:**
   - **Tier 1 (RAM Preview):** Dispatch volatile preview packets (e.g., direct mode or single-frame `04 13` packets without `04 02` commit) throttled to 15–30 Hz with drop-behind coalescing.
   - **Tier 2 (Flash Commit):** Debounce flash commit commands (`04 02`) by 500ms–2000ms after the last user interaction, or require an explicit user "Save to Keyboard" action.
2. **Low-Battery Lockout:** Query the battery percentage and power source before executing any flash commit. If battery is $<20\%$ and the keyboard is not connected via USB wired cable, block all flash writes and return a clear safety error.
3. **Dirty Sector Tracking:** Maintain an in-memory shadow copy of the keyboard's configuration. Compare buffers before sending write commands; skip writes entirely if the payload is identical (`isDirty == false`).

**Warning signs:**
- Command latency suddenly jumps from ~15ms to >200ms (indicating synchronous sector erase cycles).
- Settings stop persisting across power cycles (indicating a worn-out flash sector).
- RGB lighting flickers off for a split second whenever a color change is dispatched.

**Phase to address:**
Phase 4: Configuration & Ambient RGB Engine (with safety rails established in Phase 2)

---

### Pitfall 3: macOS TCC Input Monitoring Collisions & App Sandbox Lockdown

**What goes wrong:**
1. **TCC Permission Denial:** Attempting to open the keyboard's HID interface using standard tools or `IOHIDManager` fails with `kIOReturnNotPermitted` (`0xE00002E2`), silently strips feature reports, or triggers an intrusive macOS system dialog: *"MonKey would like to receive keystrokes from any application"*.
2. **WebHID Inaccessibility:** In web environments, Chromium grants access to the dedicated vendor collection (`0xFF68`) but completely hides and blocks the configuration interface (`0xFFFF`) because it shares a physical USB interface with `Usage Page 0x0C` (Consumer Control) and `Usage Page 0x01` (Mouse).
3. **App Sandbox Hard Block:** When packaging the driver into a sandboxed macOS application (such as Tauri v2 with App Sandbox enabled for Mac App Store or compliant enterprise distribution), `IOHIDManager` fails to enumerate or communicate with keyboard endpoints without specific entitlements.

**Why it happens:**
macOS employs Transparency, Consent, and Control (TCC) to protect users from keyloggers. Apple strictly blocks unprivileged user-space access to `Usage Page 0x01` (Generic Desktop / Keyboard / Mouse) and co-resident interfaces. While pure vendor usage pages (`Usage Page >= 0xFF00`) are exempt from Input Monitoring, composite keyboards bundle vendor configuration reports into the same interface descriptor as multimedia and mouse keys.

**How to avoid:**
1. **Dual-Interface Isolation:**
   - Explicitly separate **Interface A** (`Usage Page 0xFF68`, `Usage 0x61` — standalone bulk vendor collection) from **Interface B** (`Usage Page 0xFFFF`, `Usage 0x01` — configuration collection co-resident with Consumer/Mouse).
   - Direct all high-throughput LCD transfers strictly to Interface A (`0xFF68`). This collection never collides with protected inputs and bypasses TCC completely.
   - Route RGB and configuration writes strictly to Interface B (`0xFFFF`), using native `hidapi` / IOKit backends.
2. **Pre-flight Permission Diagnostics:** Before dispatching configuration commands, execute non-blocking permission pre-flight checks (`doctor` / `permission-status`). Provide actionable, human-readable terminal output instructing the user how to grant Input Monitoring if Interface B is blocked.
3. **App Sandbox Strategy for Tauri v2:**
   - Maintain a decoupled architecture: the core driver library (`monkey-core`) must remain independent of UI bindings.
   - For sandboxed Tauri distribution, rely on a dedicated local CLI/daemon architecture or configure specific USB hardware entitlements (`com.apple.security.device.usb`), validating IOKit device matching strings early in Phase 1.

**Warning signs:**
- `IOHIDManagerOpen` returns `0xE00002E2` on macOS Sonoma/Sequoia.
- `webhid_tester.html` can open the LCD pipe but displays zero Feature Reports for the configuration pipe.
- CLI runs perfectly in standard Terminal, but fails when executed inside a sandboxed bundle or sub-process.

**Phase to address:**
Phase 1: Architecture, Device Identification & Transport Foundation (and Phase 5 for Sandbox hardening)

---

### Pitfall 4: USB Full-Speed Bus Saturation & MCU FIFO Overrun during Bulk LCD Streaming

**What goes wrong:**
Streaming a 128x128 RGB565 frame (32,768 bytes per frame) causes dropped keystrokes, input lag, screen tearing, partial frame rendering, or total USB bus stall where the keyboard unbinds from the host OS.

**Why it happens:**
1. **Chunk Sizing Discrepancy:** Previous community reverse-engineering assumed 64-byte chunks (586 packets per frame). Real hardware probing proves that Interface `0xFF68` utilizes **4096-byte OUT reports** (8 chunks per frame).
2. **USB Full-Speed Limits:** The keyboard operates at USB Full-Speed (12 Mbps), where the maximum packet size at the USB hardware layer is 64 bytes. When the host OS sends a 4096-byte HID report, the USB host controller bursts 64 USB packets across multiple 1ms frames. Flooding 4096-byte reports back-to-back saturates the bus bandwidth, starving the keyboard's Interrupt IN endpoint (`bInterval = 1ms`) used for matrix key scanning.
3. **MCU SPI Bottleneck:** The MCU transfers pixel data from its internal SRAM buffer to the LCD driver IC (ST7789V / GC9A01) via 4-wire SPI at 20–40 MHz. Writing 4096 bytes over SPI takes approximately 1–2ms. If the host blasts chunk $N+1$ before the MCU completes the SPI DMA transfer for chunk $N$, the MCU's USB hardware FIFO overflows, corrupting pixel memory or crashing the RTOS/main loop.

**How to avoid:**
1. **Target Balanced Frame Rates:** Cap animation playback at **10–15 FPS** (66ms–100ms inter-frame interval). This provides smooth ambient status visualization without exceeding USB bus limits.
2. **Paced Chunk Transmission:** Enforce an inter-chunk pacing delay (calibrated via `monkey bench`, typically 3ms–8ms, or 5ms–10ms conservative; resulting in total wire transfer latency of ~40ms–75ms). This avoids overflowing MCU SPI DMA buffers while keeping total static display update latency under 100ms. Synchronize with the 64-byte IN report ACK on `0xFF68` if supported by firmware.
3. **Single-Flight Command Serialization:** Maintain a half-duplex command queue. Never permit RGB configuration feature reports (`0xFFFF`) and bulk LCD OUT reports (`0xFF68`) to interleave simultaneously on the USB pipe.
4. **Pre-allocated Double Buffering:** Zero-copy pipeline in `monkey-core` using static frame buffers (`[u8; 32768]`) to eliminate memory allocation latency and GC pauses.

**Warning signs:**
- Keystrokes typed while an animation is playing on the LCD are delayed, duplicated, or lost.
- LCD displays a visible diagonal tear line or horizontal color noise across chunk boundaries.
- Benchmark command (`monkey bench`) reports USB write timeout errors (`ETIMEDOUT` / `kIOReturnTimeout`).

**Phase to address:**
Phase 3: Bulk LCD Streaming Engine & Bus Management

---

### Pitfall 5: Incomplete Protocol Transactions & Firmware State Locking (`04 18` -> `04 02` -> `04 F0`)

**What goes wrong:**
Sending an isolated configuration command (such as updating keymap or RGB mode) causes the command to be silently ignored, or worse, leaves the keyboard in an unresponsive "programming state" where physical key scanning is halted until the keyboard is physically unplugged and power-cycled.

**Why it happens:**
The HFD/RKGK890 firmware implements a multi-packet state machine:
$$\text{Transaction Start } (\texttt{04 18}) \longrightarrow \text{Payload / Tables } (\texttt{04 13 / 04 20}) \longrightarrow \text{Commit } (\texttt{04 02}) \longrightarrow \text{Transaction End } (\texttt{04 F0})$$
If a client crashes, panics, drops connection, or omits the closing `04 F0` packet, the MCU's state machine remains locked inside its configuration handler, blocking normal matrix scanning and USB input reports.

**How to avoid:**
1. **Best-Effort RAII Transaction Guard & Signal Handling:** Implement a Rust `TransactionGuard` struct in `monkey-core`. The guard emits `04 18` on construction, and implements the `Drop` trait to attempt dispatching `04 F0` during normal stack unwinding. Note that Rust `Drop` is strictly **best-effort**: if the user issues `Ctrl+C` (SIGINT), the OS terminates the process without invoking `Drop` unless an explicit signal handler (such as the `ctrlc` crate) intercepts SIGINT to trigger cooperative cleanup. Furthermore, if the USB cable is unplugged, the device handle is dead and `hid_write` will return an error; since `drop(&mut self)` cannot return a `Result`, cleanup errors cannot be propagated.
2. **Dedicated Recovery Command (`monkey reset`):** Provide a standalone recovery command and session initialization check that sends a clean `04 F0` sequence when establishing communication to clear any stuck state left by hard aborts or crashes.
3. **Atomic Abort on Error:** If any intermediate chunk fails transmission or times out, immediately attempt to dispatch an explicit abort sequence (`04 F0`) before unwinding and returning the error to the caller.
4. **Sequential Execution Queue:** Serialize all hardware transactions through a single worker channel or mutex; prevent concurrent threads from initiating interleaved transactions.

**Warning signs:**
- Keyboard works for exactly one CLI command, after which all subsequent commands fail with timeouts.
- Keys stop typing on the host computer immediately following a failed driver invocation.
- Physical unplug and replug is the only way to restore keyboard functionality.

**Phase to address:**
Phase 2: Protocol Safety Rails & Schema-Driven Matrix

---

### Pitfall 6: Phantom Dongle & Multi-Mode Wireless Transport Deception

**What goes wrong:**
When the 2.4GHz wireless USB dongle is plugged into the Mac while the physical keyboard is powered off, sleeping, or switched to Bluetooth mode, the operating system enumerates the USB dongle as a valid, connected keyboard. The driver attempts to write packets, which are either swallowed silently by the dongle's radio controller or hang until timeout, leading to inaccurate diagnostic reports and failed synchronizations.

**Why it happens:**
The 2.4GHz receiver is an active USB endpoint that maintains power and OS enumeration regardless of whether the RF link to the keyboard is active. Furthermore, 3-mode keyboards often expose different HID interface configurations across USB wired mode, 2.4GHz dongle mode, and Bluetooth LE mode.

**How to avoid:**
1. **Active Heartbeat Handshake:** Never infer keyboard readiness merely from OS enumeration. Immediately execute an active query handshake (e.g., query firmware version or battery status) with a strict timeout (200ms–300ms).
2. **Explicit Connection State Machine:** Model transport states as `WiredConnected`, `DongleLinked`, `DongleUnpairedOrSleeping`, and `Disconnected`.
3. **Read-Before-Write Synchronization:** When a connection is established, read the keyboard's current hardware state instead of pushing cached software configurations, preventing accidental overwriting of adjustments made via physical Fn key shortcuts.

**Warning signs:**
- App indicates "Device Connected", but LED changes and LCD updates have zero physical effect.
- Read commands consistently time out despite the USB device node being open.
- Switching between Bluetooth and 2.4G causes the driver to crash or report corrupted descriptors.

**Phase to address:**
Phase 1: Architecture, Device Identification & Transport Foundation

---

### Pitfall 7: Report ID Byte Shift & RGB565 Endianness Inversion

**What goes wrong:**
1. **Byte Shift / Alignment Bug:** All sent packets are offset by 1 byte, causing the MCU to misinterpret the magic byte and command ID. Commands fail silently or return NAK.
2. **Color Channel Inversion:** The LCD screen renders inverted or garbled colors (e.g., pure red displays as blue or green, grayscale displays with colored banding, or checkerboard noise appears).

**Why it happens:**
- **Report ID Alignment:** On Windows `HidD_SetFeature`, the API requires byte 0 of the buffer to contain the Report ID (even if `0x00`), which the OS driver strips before sending over the USB wire. In contrast, on macOS `IOHIDDeviceSetReport` / `hidapi`, passing a buffer with Report ID `0x00` may transmit the buffer verbatim or prepend depending on the specific wrapper implementation. Documenting Byte 0 as `Report ID` and Byte 1 as `Magic` introduces off-by-one errors across operating systems.
- **RGB565 Endianness:** ST7789 LCD controllers require RGB565 data in Big-Endian format (`[R4..R0 G5..G3, G2..G0 B4..B0]`), whereas many PC graphics libraries generate Little-Endian RGB565 (`[G2..G0 B4..B0, R4..R0 G5..G3]`). Additionally, adjacent keyboard families (e.g., Sonix SN32F) use Little-Endian pixel encoding, leading developers to copy the wrong byte-order logic.

**How to avoid:**
1. **Transport-Level Report Normalization:** In `monkey-core`, strictly abstract Report ID handling at the transport layer. For Report ID 0, verify wire format explicitly using Wireshark packet captures.
2. **Diagnostic Test Patterns:** Implement a dedicated calibration command in `monkey-cli` (`monkey lcd test-pattern`) that renders pure Red (`0xF800`), pure Green (`0x07E0`), pure Blue (`0x001F`), and a 4-step grayscale ramp. Physical verification on hardware instantly flags byte-swapping issues before building complex animation engines.
3. **Strict Unit Tests for Color Pipelines:** Write unit tests for pixel packing and unpacking routines, explicitly verifying Big-Endian vs Little-Endian byte serialization against known test vectors.

**Warning signs:**
- Red UI colors appear as cyan or blue on the keyboard LCD.
- Wireshark captures show command bytes shifted to the right by 1 index (e.g., `00 04 13` instead of `04 13`).
- Firmware rejects commands that appeared correct in documentation.

**Phase to address:**
Phase 1: Transport Foundation & Phase 3: Bulk LCD Streaming Engine

---

### Pitfall 8: Readback Verification False Failures (Device PWM Rendering & Passive Scan Traps)

**What goes wrong:**
1. **False Rollback Loops:** Driver software writes an RGB configuration, immediately reads back the hardware table to verify, and asserts `sent == received`. Because the readback values differ slightly (e.g., sending `0xFF, 0x00, 0x00` reads back as `0xFD, 0x7F, 0x00`), the driver assumes write failure, triggers an error, and rolls back the UI.
2. **Passive Feature Scan Deadlock:** Probing feature reports passively across report IDs `0x00..0xFF` returns successful status codes but all 64 bytes are zero, leading researchers to conclude feature reports are unsupported.

**Why it happens:**
1. **Internal PWM/Gamma Conversion:** The keyboard MCU does not store raw RGB values verbatim; it translates RGB triplets through internal gamma correction curves or PWM duty-cycle tables before storing them in hardware registers.
2. **Active Query Requirement:** On HFD/RKGK890 firmware, the MCU will not populate feature or input report buffers passively. An explicit query trigger command (such as `04 F5` with chunk count) must be dispatched immediately prior to reading reports. Furthermore, readback data is returned via Interrupt IN reports (`ReadFile`), not `HidD_GetFeature`.

**How to avoid:**
1. **Fuzzy / ACK-Based Verification:** Verify write success by asserting transaction ACK status and sequence parity, rather than strict byte equality against analog PWM color registers.
2. **Active Query Sequence:** Always dispatch the explicit trigger packet (`04 F5 ...`) before calling `get_input_report` / `ReadFile`.
3. **Documented Behavioral Quirks:** Document hardware quirks directly in the codebase so future contributors do not introduce brittle assertions.

**Warning signs:**
- Driver logs report "Write failed: expected 0xFF0000 but received 0xFD7F00", despite the physical keyboard displaying the correct red color.
- All passive feature reads return buffers filled with `0x00`.

**Phase to address:**
Phase 2: Protocol Safety Rails & Phase 4: Configuration Engine

---

## Technical Debt Patterns

Shortcuts that seem reasonable during early reverse-engineering but create severe long-term stability and safety risks.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|:---|:---|:---|:---|
| **Direct raw HID writes from CLI commands** | Fast prototyping; no need to build a formal queue or transport layer | Race conditions, bus contention, interleaved packets that freeze MCU | Only in throwaway single-file scratch scripts (`scratch/`) |
| **Assuming 64-byte chunks for LCD streaming** | Reuses standard HID report buffers without implementing bulk report logic | 8x overhead; sending 586 packets per frame instead of 8, causing bus lag | **NEVER** (Hardware explicitly uses 4096-byte OUT reports) |
| **Using `thread::sleep` for inter-packet pacing** | Simple one-line delays between packets | Millisecond pacing jitter, thread blocking under system load | Acceptable in Phase 1 initial probes; replace with timer-based pacing |
| **Skipping transaction markers (`04 18` / `04 F0`)** | Eliminates 2 roundtrips per command | MCU firmware locks up intermittently; requires physical USB replug | **NEVER** |
| **Hardcoding layout offsets instead of schema** | Avoids writing XML/JSON layout parser | Brittle code; breaks on different board revisions or layout variants (67 vs 81 keys) | Acceptable in Milestone 1 prototype if scoped strictly to Monka 3075 |
| **Committing to flash on every UI color change** | Easy state persistence; no need for dual-tier RAM/Flash architecture | Destroys MCU SPI NOR flash endurance within weeks of regular use | **NEVER** |
| **Relying solely on WebHID for full keyboard control** | Zero-install web interface without building desktop binaries | Cannot configure RGB, remap keys, or run background daemons on macOS | Only acceptable for LCD test harness; production driver requires native Rust |

---

## Integration Gotchas

Common mistakes when interacting with operating system HID subsystems, USB stacks, and sandboxing layers.

| Integration | Common Mistake | Correct Approach |
|:---|:---|:---|
| **macOS IOKit (`hidapi`)** | Opening device with `kIOHIDOptionsTypeSeizeDevice` (exclusive access) on composite keyboard | Open with `kIOHIDOptionsTypeNone`. Target specific vendor usage pages (`0xFF68` / `0xFFFF`) to avoid permissions collisions with system keyboard drivers |
| **macOS App Sandbox (Tauri v2)** | Invoking raw IOKit or libusb calls directly inside sandboxed frontend/backend | Structure `monkey-core` to communicate via local IPC with an unprivileged helper daemon, or declare strict USB hardware entitlements (`com.apple.security.device.usb`) |
| **WebHID API (Chromium)** | Attempting to write feature reports to Interface B on macOS | Recognize that macOS/Chromium strips feature reports on co-resident interfaces. Use WebHID strictly for Interface A (`0xFF68` bulk LCD) |
| **Windows HID (`HidD_SetFeature`)** | Omitting the leading Report ID byte in the buffer passed to Win32 API | Always allocate buffer of size `ReportLength + 1`, placing Report ID in Byte 0. On wire, the OS strips byte 0 for Report ID 0 |
| **Linux `udev` Permissions** | Running application as `root` or failing to access `/dev/hidrawX` | Install `/etc/udev/rules.d/99-monka.rules` granting `uaccess` tag to `ATTRS{idVendor}=="05ac", ATTRS{idProduct}=="024f"` |
| **USB 2.4GHz Dongle vs Wired** | Assuming device path is identical across wired and wireless dongles | Match device by specific Interface Index and Usage Page rather than OS device path; execute active heartbeat ping |

---

## Performance Traps

Patterns that appear to function during single-packet tests but fail catastrophically during live animation or rapid interactions.

| Trap | Symptoms | Prevention | When It Breaks |
|:---|:---|:---|:---|
| **High FPS LCD Video Streaming (>20 FPS)** | Keystroke latency, dropped matrix inputs, LCD screen tearing | Cap animation rendering at 10–15 FPS; insert 5–10ms delay between 4096-byte chunks | Breaks at >20 FPS on Full-Speed USB (12 Mbps) |
| **Synchronous HID I/O on UI/Main Thread** | UI freezes, beachball cursor on macOS during LCD uploads | Run all HID transport I/O on dedicated background worker thread with `crossbeam-channel` or `std::sync::mpsc` channels | Breaks immediately during any 32KB LCD frame transfer |
| **Color Picker Event Flooding** | Packet backlog, delayed lighting response, MCU buffer overflow | Throttle color picker events to 30 Hz using drop-behind coalescing (discard stale pending frames) | Breaks when user drags color picker continuously for >1 second |
| **Dynamic Heap Allocation in Frame Loop** | Memory fragmentation, GC pauses, jittery animation playback | Pre-allocate frame buffers (`[u8; 32768]`) and reuse them across render loops | Breaks during continuous background animation playback |
| **Concurrent Command Interleaving** | Packets corrupt each other, MCU enters undefined state | Implement a single-flight mutex/queue; only one transaction in flight at any millisecond | Breaks when ambient RGB updates occur during LCD animation streaming |

---

## Security Mistakes

Domain-specific security, privacy, and hardware safety issues.

| Mistake | Risk | Prevention |
|:---|:---|:---|
| **Unvalidated Scancode / Keymap Writes** | Corrupts matrix layout table, rendering keys inoperable or mapping keys to dangerous codes (e.g. infinite shutdown) | Enforce boundary validation: `key_index < 81`, scancodes strictly whitelisted against `layout_81keys.json` |
| **Requiring Root / Sudo Privileges** | Exposing system to root-level exploits, violating standard user-space security practices | Target vendor usage pages (`0xFF68`/`0xFFFF`) which do not require root privileges; configure udev rules on Linux |
| **Arbitrary Bytecode Execution in Macros** | Malicious profiles executing unintended system actions or keystroke injection | Validate macro byte streams into a strictly typed Abstract Syntax Tree (AST) before serializing to hardware bytecode |
| **Unprotected DFU / Firmware Flash Commands** | Accidental invocation bricks hardware permanently | Isolate all firmware update and bootloader opcodes behind hardcoded compile-time gates and multi-step confirmation flags |

---

## UX Pitfalls

Common user experience failures in custom keyboard drivers.

| Pitfall | User Impact | Better Approach |
|:---|:---|:---|
| **Cryptic Permission Failures** | Application crashes or displays "Device Open Error: 0xE00002E2", leaving user bewildered | Detect macOS TCC blocks proactively; display actionable guide with button to open System Settings $\rightarrow$ Privacy |
| **Overwriting Keyboard on Connect** | App startup wipes custom color/layout changes made via physical Fn key shortcuts | Always perform read-before-write synchronization; treat hardware as source of truth unless user explicitly pushes |
| **Unresponsive UI During LCD Upload** | User thinks app has crashed, unplugs USB cable mid-write, and corrupts device | Display a deterministic progress bar (chunk 1/8 to 8/8) with explicit warning: *"Writing to display... Do not disconnect"* |
| **Stale Ambient Status Indicators** | Desktop app shows "Agent Busy" on LCD, but keyboard is asleep or out of range | Implement heartbeat ping; switch UI and LCD to "Offline / Sleeping" when heartbeat fails twice |

---

## "Looks Done But Isn't" Checklist

Critical verification checks to perform before declaring any milestone or subsystem complete:

- [ ] **Dual Interface Transport:** Often opens the first enumerated HID node — verify that Interface A (`0xFF68`) is opened specifically for LCD and Interface B (`0xFFFF`) is opened specifically for RGB/Config.
- [ ] **Transaction State Machine:** Often works for single commands — verify that `04 18` (start), `04 02` (commit), and `04 F0` (end) are sent across error conditions, panics, and cancellations via RAII guards.
- [ ] **LCD Color Fidelity:** Often looks correct on black-and-white graphics — verify with pure Red (`#FF0000`), pure Green (`#00FF00`), and pure Blue (`#0000FF`) test patterns to confirm RGB565 byte order (Big-Endian vs Little-Endian).
- [ ] **Concurrent Typing During LCD Playback:** Often tested on an idle desk — verify that rapid typing (100+ WPM) during a 15 FPS LCD animation produces zero dropped keystrokes, stutter, or USB timeouts.
- [ ] **RAM Preview vs Flash Commit:** Often saves every color adjustment to flash — verify that rapid color picker dragging does not trigger flash sector erase cycles, and that settings persist only after debounced release.
- [ ] **3-Mode Hotplug & Sleep Recovery:** Often tested with permanent wired USB — verify that unplugging cable, toggling wireless switches, or waking Mac from sleep recovers connection cleanly without restarting the driver.
- [ ] **TCC Exemption Verification:** Verify that the standalone CLI binary runs and controls LCD display without prompting for macOS Input Monitoring permissions.

---

## Recovery Strategies

Procedures to recover hardware and software when pitfalls are encountered.

| Failure Mode | Recovery Cost | Recovery Steps |
|:---|:---:|:---|
| **MCU Stuck in Transaction / Lockup** | LOW | 1. Terminate driver process.<br>2. Run `monkey reset` (dispatches raw `04 F0` end transaction packet).<br>3. If unresponsive, physically disconnect USB cable, wait 3 seconds, and reconnect. |
| **Corrupted Volatile Lighting State** | LOW | Dispatch `04 13` with default static white or factory preset effect. Re-sync in-memory state. |
| **Corrupted Keymap / Disabled Keys** | MEDIUM | 1. Use mouse or built-in Mac keyboard to launch CLI.<br>2. Run `monkey keymap restore-default` (writes clean `layout_81keys.json` matrix table).<br>3. Or hold physical hardware factory reset shortcut (typically `Fn + Space` or `Fn + Esc` for 3–5 seconds). |
| **macOS Permission Revocation** | LOW | 1. Open `System Settings` $\rightarrow$ `Privacy & Security` $\rightarrow$ `Input Monitoring`.<br>2. Toggle MonKey permission off and back on.<br>3. Restart terminal session. |
| **Device Stuck in Bootloader / ISP (`0x7140`)** | HIGH | 1. Do not unplug power.<br>2. Launch official vendor updater (`MK3075ProDriver V1.0.exe` or Sonix ISP tool) in a Windows VM with USB passthrough.<br>3. Reflash stock factory firmware payload from extracted PE resources. |
| **Corrupted Flash Sector (Permanent Brick)** | CRITICAL | 1. Requires disassembling keyboard case.<br>2. Solder SWD debug wires (`SWDIO`, `SWCLK`, `GND`, `3V3`) to PCB test points.<br>3. Use J-Link or ST-Link probe with OpenOCD to flash factory binary dump. |

---

## Pitfall-to-Phase Mapping

Direct mapping of domain pitfalls to roadmap phases and verification criteria.

| Pitfall | Prevention Phase | Verification Strategy |
|:---|:---|:---|
| **Bootloader/DFU Collision (Bricking)** | Phase 2: Protocol Safety Rails | Strict opcode schema validation unit tests; reject unmapped commands; verify no ISP opcodes exist in codebase |
| **macOS TCC & Sandbox Blocks** | Phase 1: Architecture & Transport | Run test CLI on clean macOS install without Input Monitoring; verify `0xFF68` opens and writes successfully |
| **Phantom Dongle Deception** | Phase 1: Architecture & Transport | Test connect/disconnect cycles with 2.4G dongle plugged in while keyboard is off; verify state machine transitions |
| **USB Bus Saturation / LCD Tearing** | Phase 3: Bulk LCD Streaming Engine | Benchmark throughput with `monkey bench`; verify stable 10–15 FPS; verify 0 dropped keystrokes while streaming |
| **RGB565 Endianness & Byte Shift** | Phase 3: Bulk LCD Streaming Engine | Render pure Red/Green/Blue test patterns via `monkey lcd test-pattern`; inspect physical LCD screen colors |
| **Incomplete Transactions (`04 18`..`04 F0`)** | Phase 2: Protocol Safety Rails | Trigger simulated panic/SIGINT during packet transmission; verify `TransactionGuard` executes `04 F0` on drop |
| **Flash Wear-Out & Brownouts** | Phase 4: Configuration & Ambient RGB | Stress-test color picker for 60 seconds; verify zero flash commits during drag; verify flash commit is debounced |
| **Readback False Failures** | Phase 4: Configuration & Ambient RGB | Verify RGB readback routine uses ACK/transaction status and fuzzy PWM matching rather than exact byte equality |

---

## Sources

- **Shenzhen HFD Technology Co., Ltd.:** Extracted `device.xml`, `KeyboardLayout.xml`, and disassembled MFC binary routines from `vendor_driver/DeviceDriver.exe` (`RKGK890`).
- **`rcsn01/GMK-67-Driver`:** Prior art reverse-engineering on identical `05AC:024F` VID/PID and `RKGK890` solution, decoding `04 18`, `04 13`, `04 20`, `04 02`, `04 F0` transaction chains.
- **Physical Hardware Measurements (2026-09-13):** Direct Chrome WebHID descriptor probe on macOS Sonoma verifying Interface A (`0xFF68`, OUT 4096 / IN 64) and Interface B (`0xFFFF`, co-resident Consumer/Mouse).
- **OpenRGB & Sonix-QMK Projects:** Documentation on Sonix/HFD SN32F248B Cortex-M0 microcontrollers, ISP bootloader PID `0x7140` collision hazards, and flash sector endurance.
- **Apple Developer Documentation:** IOKit HID Device Access, macOS TCC Input Monitoring policies (`kTCCServiceListenEvent`), and App Sandbox USB hardware entitlements.

---
*Pitfalls research for: MonKey (Monka 3075 Pro / HFD RKGK890)*
*Researched: 2026-09-13*
