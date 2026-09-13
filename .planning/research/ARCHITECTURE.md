# Architecture Research

**Domain:** Hardware Driver & Tooling (Rust / macOS USB HID / Embedded OEM Peripheral)
**Researched:** 2026-09-13
**Confidence:** MEDIUM overall; OEM identity/layout are HIGH, while transport and protocol details remain reported or INFERRED

> **Evidence boundary:** This is an architecture proposal, not a hardware-capture report. The working tree contains OEM XML and research notes, but no raw USB capture (`.pcap`/`.pcapng`). The `0xFF68`/`0xFFFF` split, 4096-byte LCD transfer, feature-report size, opcode behavior, ACK behavior, and RAM-versus-Flash behavior must not be treated as verified target-board facts.

---

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          User & Applications Layer                          │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────┐     ┌────────────────────────────────┐  │
│  │   monkey-cli (Terminal CLI)   │     │  Tauri v2 Desktop / Menu Bar   │  │
│  │   (info, lcd, rgb, bench)     │     │      (Future Milestone)        │  │
│  └───────────────┬───────────────┘     └───────────────┬────────────────┘  │
│                  │                                     │                   │
├──────────────────┴─────────────────────────────────────┴───────────────────┤
│                     monkey-core: High-Level Driver Layer                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         KeyboardDriver Session                        │  │
│  │  - Device Lifecycle & Reconnect     - Single-Flight Command Queue     │  │
│  │  - RAM Preview vs Flash Commit       - Heartbeat / Keep-Alive Monitor  │  │
│  └───────────────┬─────────────────────────────────────┬─────────────────┘  │
│                  │                                     │                    │
│  ┌───────────────▼───────────────┐     ┌───────────────▼────────────────┐  │
│  │       LcdRenderingEngine      │     │       RgbLightingEngine        │  │
│  │  - RGB565 Endian/Dithering    │     │  - Single-Packet Ambient Mode  │  │
│  │  - 32KB Frame 8-Chunk Slicer  │     │  - Per-Key Matrix Mapping      │  │
│  │  - 10-15 FPS Frame Regulator  │     │  - Flash Wear Throttling       │  │
│  └───────────────┬───────────────┘     └───────────────┬────────────────┘  │
│                  │                                     │                    │
├──────────────────┴─────────────────────────────────────┴───────────────────┤
│                   monkey-core: Protocol & Safety Gate Layer                 │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    CapabilityMatrix & SafetyGate                      │  │
│  │  - Device Identification (Model + HW Rev + FW Ver + Transport)        │  │
│  │  - Opcode Whitelist & Parameter Boundary Validation                   │  │
│  │  - Bootloader Device Isolation (Reject ISP PID e.g. 0x7140)          │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │                                      │
│  ┌───────────────────────────────────▼───────────────────────────────────┐  │
│  │                          ProtocolCodec Layer                          │  │
│  │  - 64-Byte Feature Report Codec      - 4096-Byte Bulk Frame Codec     │  │
│  │  - Checksum Engines (CRC16/Sum/XOR)  - Magic (0x04) & AA55 Framing    │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │                                      │
├──────────────────────────────────────┴──────────────────────────────────────┤
│                     monkey-core: Transport Abstraction Layer                │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                         HidTransport Trait                            │  │
│  │   (Send/Receive Feature, Write/Read Interrupt & Bulk OUT, Enumerate)  │  │
│  └───────────────┬─────────────────────────────────────┬─────────────────┘  │
│                  │                                     │                    │
│  ┌───────────────▼───────────────┐     ┌───────────────▼────────────────┐  │
│  │    HidapiTransport (Native)   │     │      MockTransport (Tests)     │  │
│  │  - Interface A: Bulk UP 0xFF68│     │  - Deterministic in-memory bus │  │
│  │  - Interface B: Cfg  UP 0xFFFF│     │  - Zero hardware CI test suite │  │
│  └───────────────┬───────────────┘     └────────────────────────────────┘  │
│                  │                                                          │
├──────────────────┴──────────────────────────────────────────────────────────┤
│                       Operating System & Hardware Bus                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────┐     ┌────────────────────────────────┐  │
│  │  macOS IOKit / IOHIDManager   │     │  Linux hidraw / Windows Win32  │  │
│  │  (No TCC Input Monitor for    │     │  (Future cross-platform        │  │
│  │   isolated Vendor Page FF68)  │     │   support targets)             │  │
│  └───────────────┬───────────────┘     └────────────────┬───────────────┘  │
│                  │                                      │                   │
│  ┌───────────────┴──────────────────────────────────────┴───────────────┐  │
│  │ Monka 3075 Pro Hardware (Shenzhen HFD RKGK890, 05AC:024F, 128x128 LCD)│  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| `monkey-cli` | CLI entry point, argument parsing, interactive diagnostics, benchmark harness, human/JSON output formats | Rust binary (`clap` v4 derive, `tracing-subscriber`, `indicatif`) |
| `KeyboardDriver` | High-level session orchestration, device lifecycle, connection state machine, error recovery, transaction safety | Struct holding `Arc<dyn Transport>`, `CapabilityMatrix`, and `CommandQueue` |
| `CommandQueue` | Enforces single-flight serial execution, manages inter-packet timing delays (10ms-80ms), prevents MCU FIFO drops | Dedicated OS worker thread with `crossbeam-channel` or `std::sync::mpsc` |
| `CapabilityMatrix` | Evaluates hardware capabilities based on multi-axis key (`model + hw_rev + fw_ver + transport`), provides feature flags | Strongly-typed registry (`HashMap` / static lookup) returning feature bitflags & parameter bounds |
| `SafetyGate` | Hardened firewall enforcing default-deny opcode whitelisting against capture-verified commands | Typestate validator converting `RawPacket` into `ValidatedPacket` before transport write |
| `ProtocolCodec` | Serializes high-level commands into 64-byte configuration packets (magic `0x04`, `AA 55` marker) and 4096-byte bulk frames | Zero-copy byte buffers (`bytes`, `nom` / manual packing), checksum algorithms (One's Complement, Additive 16-bit, CRC16) |
| `LcdRenderingEngine` | Resizes images/GIFs to 128x128, converts pixels to RGB565, applies Floyd-Steinberg dithering, segments 32KB into 8x 4096B chunks, regulates 10-15 FPS | `image` crate processing pipeline, zero-copy chunk iterator |
| `RgbLightingEngine` | Controls built-in RGB modes (`04 13`), per-key color tables (`04 20`), manages RAM preview throttling (30Hz) vs debounced Flash commits | Key matrix mapper referencing `layout_81keys.json`, software timer for commit debouncing |
| `Transport Abstraction` | Abstracts physical OS HID communication behind clean synchronous traits, isolates platform-specific quirks | `Transport` trait, with `HidTransport` and `MockTransport` |
| `DualInterfaceManager` | Coordinates two distinct HID interfaces on Monka: Interface A (`0xFF68` bulk display) and Interface B (`0xFFFF` configuration) | Holds two distinct `hidapi::HidDevice` handles under unified orchestration |

---

## Recommended Project Structure

```
MonkaKeyboard/
├── Cargo.toml                     # Cargo workspace root
├── Cargo.lock
├── crates/
│   ├── monkey-core/               # Core driver library (UI-agnostic, embeddable)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # Public API re-exports
│   │       ├── error.rs           # DriverError enum, Result types
│   │       ├── capabilities/      # Capability matrix & hardware rules
│   │       │   ├── mod.rs         # CapabilityMatrix engine
│   │       │   ├── device_id.rs   # Model, HwRev, FwVer, TransportType
│   │       │   ├── flags.rs       # CapabilityFlags (has_lcd, rgb_per_key, etc.)
│   │       │   └── registry.rs    # Known device definitions (Monka 3075 Pro, GMK67)
│   │       ├── safety/            # Safety gate & opcode quarantine
│   │       │   ├── mod.rs         # SafetyGate validator trait & impl
│   │       │   ├── whitelist.rs   # Whitelisted opcodes & payload bounds
│   │       │   └── blacklist.rs   # Prohibited vectors (DFU, bootloader, unverified flash)
│   │       ├── protocol/          # Packet schemas, codecs, and checksums
│   │       │   ├── mod.rs         # Packet definitions
│   │       │   ├── opcodes.rs     # OpCode enum (0x18, 0x13, 0x20, 0x02, etc.)
│   │       │   ├── codec.rs       # 64-byte frame serialization & deserialization
│   │       │   ├── checksum.rs    # Checksum implementations (One's comp, CRC16, etc.)
│   │       │   └── report.rs      # ReportId definitions (0x00, 0x04, etc.)
│   │       ├── transport/         # HID transport abstraction
│   │       │   ├── mod.rs         # HidTransport trait definition
│   │       │   ├── hidapi_impl.rs # Production hidapi / IOKit transport
│   │       │   ├── dual_iface.rs  # Dual-interface coordinator (0xFF68 + 0xFFFF)
│   │       │   └── mock.rs        # MockTransport for unit testing and CI
│   │       ├── device/            # High-level keyboard session
│   │       │   ├── mod.rs         # KeyboardDriver struct
│   │       │   ├── queue.rs       # Single-flight command queue with delay profiles
│   │       │   └── session.rs     # Connection lifecycle, ping, reconnect logic
│   │       ├── lcd/               # 128x128 LCD rendering and streaming pipeline
│   │       │   ├── mod.rs         # LcdEngine coordinator
│   │       │   ├── color.rs       # RGB888 -> RGB565 converter (endian-aware)
│   │       │   ├── dither.rs      # Floyd-Steinberg error diffusion
│   │       │   ├── chunker.rs     # 32,768B frame -> 8x 4096B chunk slicer
│   │       │   └── streamer.rs    # 10-15 FPS pacing & flow control regulator
│   │       └── rgb/               # RGB lighting & per-key matrix control
│   │           ├── mod.rs         # RgbEngine coordinator
│   │           ├── mode.rs        # Built-in lighting modes & parameters
│   │           ├── matrix.rs      # Key layout & rgb_light_index mapper
│   │           └── state.rs       # RAM Preview vs Flash Commit coordinator
│   │
│   └── monkey-cli/                # Diagnostic and control CLI tool
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # CLI entry point and top-level router
│           ├── args.rs            # Clap CLI command line definitions
│           └── commands/          # Subcommand handlers
│               ├── mod.rs
│               ├── info.rs        # Probe hardware, print capability matrix
│               ├── lcd.rs         # Render image, test pattern, GIF animation
│               ├── rgb.rs         # Set built-in mode, color, brightness, speed
│               └── bench.rs       # HID throughput & latency benchmark suite
│
├── assets/                        # Static test fixtures (images, layouts)
│   ├── layout_81keys.json         # 81-key matrix and scancode map
│   └── sample_128x128.png         # Test image for LCD validation
└── .planning/                     # Project planning & research docs
```

### Structure Rationale

- **`crates/monkey-core` standalone library:**
  Hardware drivers must not depend on UI or CLI runtimes. Isolating all hardware protocol, safety validation, and rendering logic in `monkey-core` allows exact reuse across `monkey-cli` today and a Tauri v2 desktop application tomorrow without code duplication.
- **`transport/` abstraction with `mock.rs`:**
  macOS USB development often requires physical hardware plugged in. Providing a first-class `MockTransport` allows rigorous unit testing of packet codecs, LCD chunking, and capability filtering in headless CI pipelines (macOS/Linux runners) without real hardware.
- **`safety/` quarantined from `protocol/`:**
  Decoding bytes is distinct from permitting bytes. The `protocol/` module translates between Rust types and byte arrays, while `safety/` enforces that constructed packets comply with hardware safety policies. This eliminates accidental bypass of safety gates.
- **`lcd/` and `rgb/` domain isolation:**
  Display transmission operates on a completely different interface (`0xFF68`, 4096-byte OUT chunks) with different bandwidth characteristics than RGB configuration (`0xFFFF`, 64-byte feature reports). Keeping them in separate submodules prevents logic bleeding and enables independent optimization.

---

## Architectural Patterns

### Pattern 1: Safety Gate with Typestate & Schema Whitelist

**What:**
A compile-time and runtime validation barrier that prevents raw, arbitrary byte buffers from being transmitted to the keyboard. Packets start in an `UncheckedPacket` state and can only be promoted to `ValidatedPacket` by passing through `SafetyGate::validate()`.

**When to use:**
Whenever generating packets for OEM devices. To prevent catastrophic bricking or unintended state corruption, all outbound packets must be whitelisted against empirical target-board captures. USB bootloader PID isolation (e.g., rejecting ISP PID `0x7140` or `0xFFFF`) is strictly separated at the device discovery/enumeration layer.

**Trade-offs:**
- *Pros:* Mathematically eliminates hardware bricking vectors caused by guessing opcodes or buffer overflows.
- *Cons:* Requires explicit schema entries for every supported command; unverified commands cannot be sent without modifying the whitelist.

**Example:**
```rust
pub struct UncheckedPacket {
    pub report_id: u8,
    pub payload: Vec<u8>,
}

pub struct ValidatedPacket {
    report_id: u8,
    payload: Vec<u8>,
}

impl ValidatedPacket {
    pub fn as_bytes(&self) -> &[u8] {
        &self.payload
    }
    pub fn report_id(&self) -> u8 {
        self.report_id
    }
}

pub struct SafetyGate<'a> {
    matrix: &'a CapabilityMatrix,
}

impl<'a> SafetyGate<'a> {
    pub fn new(matrix: &'a CapabilityMatrix) -> Self {
        Self { matrix }
    }

    pub fn validate(&self, packet: UncheckedPacket) -> Result<ValidatedPacket, SafetyViolation> {
        if packet.payload.is_empty() {
            return Err(SafetyViolation::EmptyPayload);
        }

        let magic = packet.payload[0];
        let cmd = packet.payload.get(1).copied().unwrap_or(0);

        // Strict default-deny whitelist: only allow opcodes verified in capture artifacts.
        // Opcode validation is cleanly separated from device enumeration (e.g. bootloader PID isolation).
        if !self.matrix.is_opcode_permitted(magic, cmd) {
            return Err(SafetyViolation::UnsupportedOpcode(magic, cmd));
        }

        // Payload length validation
        if packet.payload.len() != 64 && packet.payload.len() != 4096 {
            return Err(SafetyViolation::InvalidLength(packet.payload.len()));
        }

        Ok(ValidatedPacket {
            report_id: packet.report_id,
            payload: packet.payload,
        })
    }
}
```

### Pattern 2: Dual-Interface Split-Transport Pattern

**What:**
Encapsulates the keyboard's two physically separate HID entries into a unified transport coordinator. Interface A (`0xFF68`) handles bulk LCD frame transfers via 4096-byte OUT reports without macOS TCC permission hurdles; Interface B (`0xFFFF`) handles 64-byte feature reports for configuration.

**When to use:**
Whenever interacting with composite USB HID devices where high-bandwidth media (displays, storage) resides on a vendor usage page, while configuration shares endpoints with mouse/consumer control.

**Trade-offs:**
- *Pros:* Allows unprompted user-space LCD streaming on macOS while cleanly scoping configuration permissions. Prevents LCD bulk traffic from choking RGB configuration queues.
- *Cons:* Must manage two separate OS device handles, handle individual disconnect/reconnect events, and synchronize lifetime states.

**Example:**
```rust
pub trait Transport: Send {
    fn write_bulk(&self, chunk: &[u8]) -> Result<(), TransportError>;
    fn send_feature_report(&self, report: &[u8]) -> Result<(), TransportError>;
    fn get_feature_report(&self, report_id: u8, buf: &mut [u8]) -> Result<usize, TransportError>;
    fn is_connected(&self) -> bool;
    fn connection_mode(&self) -> ConnectionMode;
}

pub struct HidTransport {
    // Interface A: Usage Page 0xFF68 / Usage 0x61 (Bulk LCD)
    lcd_device: Option<hidapi::HidDevice>,
    // Interface B: Usage Page 0xFFFF / Usage 0x0001 (Config / Feature)
    config_device: Option<hidapi::HidDevice>,
    connection_mode: ConnectionMode,
}

impl HidTransport {
    pub fn open(api: &hidapi::HidApi, target: &DiscoveredDevice) -> Result<Self, TransportError> {
        let mut lcd_dev = None;
        let mut cfg_dev = None;

        for device_info in api.device_list() {
            if device_info.vendor_id() == target.vid && device_info.product_id() == target.pid {
                match (device_info.usage_page(), device_info.usage()) {
                    (0xFF68, 0x61) => {
                        lcd_dev = Some(device_info.open_device(api)?);
                    }
                    (0xFFFF, 0x01) => {
                        cfg_dev = Some(device_info.open_device(api)?);
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            lcd_device: lcd_dev,
            config_device: cfg_dev,
            connection_mode: target.connection_mode,
        })
    }
}
```

### Pattern 3: Single-Flight Serialized Command Queue with Inter-Packet Pacing

**What:**
A queue actor pattern ensuring that only **one configuration transaction** is active on the HID bus at any given millisecond, enforcing hardware-mandated inter-packet cooldown periods.

**When to use:**
Required on embedded keyboards powered by resource-constrained microcontrollers (e.g. ARM Cortex-M0 running at 48MHz with 8KB SRAM) whose USB FIFOs drop incoming packets if bombarded by Host traffic.

**Trade-offs:**
- *Pros:* Guaranteed packet receipt, zero buffer overflow, deterministic transaction latency.
- *Cons:* Host-side throughput is capped; caller threads must await queue turn.

**Example:**
```rust
pub enum CommandDelay {
    StandardQuery, // 10ms - 20ms
    LcdChunk,      // 2ms - 5ms inter-chunk drain
    FlashCommit,   // 50ms - 100ms flash erase/program cycle
}

impl CommandDelay {
    pub fn duration(&self) -> std::time::Duration {
        match self {
            Self::StandardQuery => std::time::Duration::from_millis(15),
            Self::LcdChunk => std::time::Duration::from_millis(3),
            Self::FlashCommit => std::time::Duration::from_millis(80),
        }
    }
}

pub struct CommandQueue {
    sender: crossbeam_channel::Sender<CommandTask>,
}

struct CommandTask {
    packet: ValidatedPacket,
    delay: CommandDelay,
    response_tx: crossbeam_channel::Sender<Result<Vec<u8>, DriverError>>,
}
```

### Pattern 4: Two-Tier State Sync: Ephemeral RAM Preview vs Debounced Flash Commit

**What:**
Separates real-time visual responsiveness from persistent flash storage writes. Fast parameter tweaks (e.g., color slider dragging, ambient agent state updates) write directly to the MCU's volatile RAM registers, while a debounced timer commits to SPI NOR Flash only when changes settle.

**When to use:**
Used for ambient lighting, live status indicators, and UI color pickers to prevent NOR Flash wearout (which is typically rated for only 10,000 to 100,000 write cycles).

**Trade-offs:**
- *Pros:* Protects hardware lifetime; provides 0ms perceived lag for interactive controls; eliminates brownout bricking risk during rapid edits.
- *Cons:* If power is cut or cable disconnected before debounce fires, changes revert to previous flash state.

---

## Data Flow

### Request Flow

```
[User / CLI Command]
        │
        ▼
[monkey-cli Router]
        │
        ▼
[KeyboardDriver Session]
        │
   (Constructs Payload)
        │
        ▼
[SafetyGate & CapabilityMatrix] ──── (Violates Whitelist) ────► [Return HardwareSafetyError]
        │
   (ValidatedPacket)
        │
        ▼
[Single-Flight CommandQueue]
        │
   (Serializes & Enforces Inter-Packet Delay)
        │
        ▼
[DualInterfaceTransport]
   ├── If Bulk Display Frame ────► Interface A (UP 0xFF68) ────► USB OUT Report (4096B)
   └── If Configuration/RGB  ────► Interface B (UP 0xFFFF) ────► USB Feature Report (64B)
        │
        ▼
[Hardware MCU / RKGK890]
```

### State Management

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           KeyboardDriver State                              │
│                                                                             │
│  ConnectionState: [Disconnected] ──► [Probing] ──► [Ready] ──► [Suspended]  │
│                                                                             │
│  Hardware State Sync:                                                       │
│  ┌───────────────────────────────┐     ┌─────────────────────────────────┐  │
│  │   RAM Preview (Live / Volatile)│    │   Flash State (Persistent)     │  │
│  │   - RGB Ambient Streaming     │     │   - Default Boot Profile        │  │
│  │   - Status Indicators         │     │   - Stored Keymap & Macros      │  │
│  │   - Throttled at 30 Hz max    │     │   - Debounced 500ms on settle   │  │
│  └───────────────────────────────┘     └─────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Data Flows

#### 1. LCD Frame Rendering & 8-Chunk Transfer Pipeline

Transferring a complete frame to the 128x128 LCD screen follows a strict, zero-copy chunking pipeline across Interface A:

```
[Source Image / GIF Frame]
        │ (1. Resize & Crop to 128x128)
        ▼
[128x128 RGBA Buffer]
        │ (2. Quantization & Floyd-Steinberg Dithering)
        ▼
[128x128 RGB565 Buffer] (128 * 128 * 2 = 32,768 Bytes)
        │
        ▼
[Chunker Iterator: 8 Chunks of 4096 Bytes]
   ├── Chunk 0: Offset 0x0000..0x1000  (Bytes 0..4096)
   ├── Chunk 1: Offset 0x1000..0x2000  (Bytes 4096..8192)
   ├── Chunk 2: Offset 0x2000..0x3000  (Bytes 8192..12288)
   ├── Chunk 3: Offset 0x3000..0x4000  (Bytes 12288..16384)
   ├── Chunk 4: Offset 0x4000..0x5000  (Bytes 16384..20480)
   ├── Chunk 5: Offset 0x5000..0x6000  (Bytes 20480..24576)
   ├── Chunk 6: Offset 0x6000..0x7000  (Bytes 24576..28672)
   └── Chunk 7: Offset 0x7000..0x8000  (Bytes 28672..32768)
        │
        ▼
[Sequential OUT Report Dispatch (Report ID 0 on Interface UP 0xFF68)]
   - Paced with empirical inter-chunk delay (estimated 3ms-8ms, yielding bus transfer ~40-75ms)
   - Streamer loop throttles frame submission to target 10-15 FPS (66ms-100ms interval)
        │
        ▼
[MCU DMA transfers buffer to ST7789V / GC9A01 LCD via 4-wire SPI]
```

1. **Host Processing:** The image is decoded into a 128x128 grid. Floyd-Steinberg dithering diffuses color quantization errors to minimize color banding on the 16-bit panel (target <15ms).
2. **Binary Packing:** The frame is converted to raw RGB565 bytes (16 bits per pixel, little-endian `[lo, hi]`). The total raw frame size is exactly $128 \times 128 \times 2 = 32,768$ bytes ($32\text{ KB}$).
3. **Chunk Segmentation:** The 32KB buffer is sliced into exactly $8 \times 4096$-byte blocks. Because the vendor collection on `0xFF68` has an OUT report size of 4096 bytes (Report ID 0), no extra payload framing overhead is required per chunk.
4. **Bus Transmission:** Chunks 0 through 7 are transmitted sequentially via `send_bulk_lcd_chunk`. An inter-chunk pacing delay (calibrated via `monkey bench`, typically 3–8ms) allows the MCU's internal SPI DMA controller to drain its ping-pong RAM buffer without dropping chunks.
5. **Rate Regulation & Bus Load:** The LCD Streamer task sleeps between frames to maintain a steady 10–15 FPS (66.6ms–100ms per frame). At 15 FPS, streaming 491.5 KB/s consumes ~3.93 Mbps on wire—roughly 32.8% of theoretical 12 Mbps Full-Speed capacity (~40–46% effective usable bus capacity after USB framing and protocol overhead). Inter-chunk pacing and frame-drop backpressure ensure keyboard input reports remain prioritized and latency unaffected.

#### 2. Ambient RGB Update Flow (Single-Packet Mode `04 13`)

For driving ambient lighting (such as reflecting AI coding agent state):

```
[Agent State / CLI Trigger: e.g. WAITING_FOR_YOU (Amber Pulse)]
        │
        ▼
[RgbLightingEngine::set_ambient_mode(mode, r, g, b, speed, brightness)]
        │
        ▼
[Construct 64-Byte Feature Payload]
   - Byte 0: 0x04 (Magic)
   - Byte 1: 0x13 (Built-in Mode Command)
   - Byte 2..4: R, G, B
   - Byte 8: Colortype
   - Byte 9..10: Speed, Brightness
   - Byte 14..15: 0xAA, 0x55 (Framing Marker)
   - Byte 62..63: Checksum
        │
        ▼
[SafetyGate::validate()] ──► Checks opcode 0x13 whitelisted, bounds valid
        │
        ▼
[CommandQueue::submit()] ──► Dispatched via Interface B (UP 0xFFFF)
        │
        ▼
[MCU updates LED PWM registers in RAM — NO Flash cycle invoked]
```

#### 3. Hardware Probing & Capability Negotiation

1. **Enumeration:** `monkey-core` scans USB devices with VID `0x05AC` and PID `0x024F`.
2. **Interface Inspection:** Verifies presence of both `0xFF68` and `0xFFFF` collections. If only `0xFF68` is found, flags the transport as wireless or restricted.
3. **Query Handshake:** Sends non-destructive read request (`04 F5` or baseline query packet).
4. **Matrix Matching:** Resolves the exact `CapabilityMatrix` record for `Monka 3075 Pro` (enabling 81 keys matrix, 128x128 LCD, and 8-chunk display transport).

#### 4. Heartbeat & Phantom Dongle Management

1. In 2.4GHz wireless operation, the USB dongle remains enumerated even when the physical keyboard is powered off or sleeping.
2. A background timer in `KeyboardDriver` issues a heartbeat query every 3–5 seconds.
3. If 2 consecutive queries timeout, the driver transitions `ConnectionState` to `Sleeping`.
4. While `Sleeping`, bulk LCD streaming and RGB updates are paused to conserve host CPU and avoid USB bus errors.
5. Upon the next successful response (user presses a key, waking the MCU), the driver transitions back to `Ready` and resynchronizes state.

---

## Scaling Considerations

| Scale Dimension | Architecture Adjustments |
|-----------------|--------------------------|
| **Single Keyboard CLI (Current Milestone)** | Monolithic `monkey-core` library + `monkey-cli` binary. Synchronous command execution or lightweight Tokio runtime. Direct HID access via `hidapi`. |
| **Multi-Device / Mixed OEM Family** | Capability matrix dynamic registry. Abstract layout files (`layout_81keys.json`, `layout_67keys.json`) loaded dynamically. Plugin-style codec modules for Sonix vs HFD packet dialects. |
| **Desktop App / Tauri v2 Integration** | Embed `monkey-core` inside Tauri backend. Hardware operations execute on dedicated synchronous worker thread; Tauri async commands communicate across channels with oneshot responses. Background tray/menubar daemon consuming <15MB RAM. |
| **Continuous Ambient Status Streaming** | Frame drop detection in `LcdRenderingEngine`. Coalescing drop-behind queue for RGB updates. Zero flash commits during continuous streaming. |

### Scaling Priorities

1. **First Bottleneck: USB HID Bus Congestion & Inter-Chunk Timing**
   - *Risk:* Flooding Interface A with 4096-byte LCD chunks at unconstrained rates causes the keyboard's internal SPI DMA buffer to overflow, leading to visual tearing or keyboard lockup.
   - *Fix:* Enforce strict inter-chunk pacing (2–3ms) in `LcdStreamer` and rate-limit frame generation to 10–15 FPS.
2. **Second Bottleneck: macOS Sandbox & TCC Permission Friction**
   - *Risk:* In macOS App Sandbox environments (Tauri v2), accessing keyboard interfaces can be blocked by `kIOReturnNotPermitted`.
   - *Fix:* Strictly isolate the vendor-defined collection `0xFF68` (unrestricted) from the configuration collection `0xFFFF`, using proper entitlements (`com.apple.security.device.usb`) and graceful degradation if configuration access is denied.

---

## Anti-Patterns

### Anti-Pattern 1: Speculative Opcode Guessing & Blind Probing

**What people do:**
Sending random opcodes (`0x00` through `0xFF`) to see what responds or scanning report IDs indiscriminately.
**Why it's wrong:**
In embedded keyboard controllers (especially Sonix/HFD), ISP bootloader entry or mass erase routines can be triggered by unintended control sequences. Guessed packets can wipe internal NOR flash and permanently brick the device.
**Do this instead:**
Use an explicit `SafetyGate` with a strict default-deny whitelist. Never send unverified opcodes. Validate commands against archived target-board captures; prior-art traces may inform hypotheses but do not establish Monka support. Quarantine dangerous USB PIDs (e.g., bootloader PID `0x7140`) at the device discovery/enumeration layer.

### Anti-Pattern 2: Monolithic Single-Pipe Transport Assumption

**What people do:**
Treating the keyboard as a single HID handle and attempting to send 4096-byte display chunks and 64-byte configuration packets over the same endpoint.
**Why it's wrong:**
Physical hardware measurements prove the Monka 3075 Pro has two completely separate interfaces: Interface A (`0xFF68`, 4096B OUT) and Interface B (`0xFFFF`, 64B Feature). Conflating them results in OS pipe errors and permission blocks.
**Do this instead:**
Employ the `DualInterfaceTransport` pattern, holding distinct handles for bulk LCD transfers and configuration reports.

### Anti-Pattern 3: Direct-to-Flash Ambient Streaming

**What people do:**
Writing ambient RGB status updates or animated LCD frames directly using flash-commit opcodes (e.g. including `04 02` save commands on every frame).
**Why it's wrong:**
Embedded NOR flash typically endures only 10,000 to 100,000 write cycles per sector. Streaming live updates to flash will permanently burn out the flash chip within a few days of continuous operation.
**Do this instead:**
Use RAM preview commands (`04 13` for RGB) and keep LCD animation streams in host memory or MCU volatile RAM buffers. Require deliberate, debounced user action before committing any setting to flash.

### Anti-Pattern 4: Unthrottled Fire-and-Forget Dispatch

**What people do:**
Spawning independent async tasks that write to `hidapi` concurrently without coordination.
**Why it's wrong:**
The underlying USB hardware has a single endpoint FIFO. Concurrent writes corrupt frame boundaries and cause packet interleaving.
**Do this instead:**
Route all outbound traffic through a single-flight serialized `CommandQueue` with strict inter-packet delay intervals.

---

## Integration Points

### External Services & OS Boundaries

| Boundary | Integration Pattern | Notes & Gotchas |
|----------|---------------------|-----------------|
| **macOS IOKit (`hidapi`)** | Native C FFI via `hidapi` crate | Interface `0xFF68` requires no special TCC permissions. Interface `0xFFFF` is co-resident with Consumer Control/Mouse; must be opened by specific path or vendor usage page to avoid raw keyboard hook restrictions. |
| **macOS App Sandbox (Future Tauri)** | Entitlements: `com.apple.security.device.usb` + `com.apple.security.app-sandbox` | The CLI runs natively without sandbox; the core driver must maintain zero filesystem/network dependencies to run inside the future sandboxed GUI container. |
| **Linux `hidraw` (Cross-Platform)** | `/dev/hidraw*` via `hidapi-rs` | Requires udev rules (`05ac:024f`, mode `0666`) to allow non-root user-space access. |
| **Windows Win32 HID** | `CreateFileW` + `HidD_SetFeature` / `WriteFile` | Windows locks keyboard usage collection (`0x01/0x06`) with sharing violations; requires connecting strictly via vendor collection handle. |

### Internal Module Boundaries

| Boundary | Communication Mechanism | Notes & Considerations |
|----------|--------------------------|------------------------|
| **`monkey-cli` ↔ `monkey-core`** | Rust function calls, `KeyboardDriver` API | CLI only handles CLI arguments, terminal formatting, and process exit codes. All device logic is strictly in `monkey-core`. |
| **`KeyboardDriver` ↔ `CommandQueue`** | Crossbeam channel or dedicated worker thread queue | Isolates caller execution from USB bus timing and inter-packet pacing. |
| **`CommandQueue` ↔ `SafetyGate`** | Synchronous validation barrier | Unchecked packets cannot enter the transport pipeline without passing validation. |
| **`LcdEngine` ↔ `Transport`** | Direct chunk streaming via `write_bulk` | Bypasses the 64-byte config queue to maximize LCD throughput over dedicated Interface A. |
| **`Future Tauri App` ↔ `monkey-core`** | Tauri IPC commands bridging to dedicated worker thread via channels | Core library driver runs on a dedicated synchronous OS thread; frontend async commands receive responses across oneshot channels. |

---

## Suggested Build Order & Dependencies

Based on component dependencies and risk reduction, the recommended implementation order strictly follows the roadmap phases (`ROADMAP.md`):

```
[Phase 1: Workspace, Transport Foundation & Device Probing]
  ├── Setup Cargo workspace (`crates/monkey-core`, `crates/monkey-cli`)
  ├── Implement `Transport` trait and `MockTransport`
  ├── Implement `HidTransport` with dual-interface discovery (`0xFF68` + `0xFFFF`)
  └── Build `monkey-cli info` and `monkey-cli probe` (DISC-01..05)
           │
           ▼
[Phase 2: Protocol Codecs, Transaction Safety Rails & Benchmark Harness]
  ├── Define `OpCode` enum, 64-byte frame layouts, and framing markers (`0x04`, `AA 55`)
  ├── Implement Checksum algorithms (One's Complement, Additive 16-bit, CRC16)
  ├── Build `CapabilityMatrix` (model, hw_rev, fw_ver, transport)
  ├── Implement `SafetyGate` whitelist validator and bricking isolation tests
  ├── Implement single-flight `CommandQueue` with inter-packet delay profiles
  └── Build `monkey-cli bench` diagnostic benchmark harness (DIAG-01, PROT-01..05)
           │
           ▼
[Phase 3: High-Performance LCD Rendering & Streaming Engine]
  ├── Implement RGB565 color conversion with Floyd-Steinberg dithering
  ├── Build 32KB frame chunker (8x 4096-byte OUT chunks)
  ├── Implement `LcdStreamer` rate regulator (10-15 FPS pacing)
  └── Build `monkey-cli lcd` (static image rendering and GIF animation player) (LCD-01..05)
           │
           ▼
[Phase 4: Ambient RGB Engine, Matrix Mapping & Two-Tier State Persistence]
  ├── Implement single-packet built-in mode control (`04 13`)
  ├── Implement per-key matrix mapper from `layout_81keys.json` (`04 20`)
  ├── Implement 2-tier RAM Preview (30Hz) vs Flash Commit (500ms debounce) synchronizer
  └── Build `monkey-cli rgb` (mode, color, brightness, speed controls) (RGB-01..04)
           │
           ▼
[Phase 5: Production Hardening, Packaging & Release Readiness]
  ├── Build `monkey-cli doctor` diagnostic command (DIAG-02)
  ├── Setup cargo-deny license/vulnerability audit & cargo-clippy pedantic CI gates
  ├── Packaging & binary release pipeline (Universal 2 macOS binary, shell completions, manpages)
  └── Non-root Linux udev rule documentation & end-to-end integration validation
```

### Build Order Implications:
1. **MockTransport First:** Implementing `MockTransport` in Phase 1 unlocks 100% automated test coverage for Phase 2 (protocol & safety gate) without requiring hardware tethering.
2. **Discovery & Probing in Phase 1:** `monkey info` and `monkey probe` validate hardware presence and physical interface mapping before protocol serialization work begins.
3. **Safety Gate & Benchmark Before Feature Streaming:** `SafetyGate` and `monkey bench` in Phase 2 ensure packet safety and measure exact physical bus pacing/timing thresholds before building high-speed LCD streaming in Phase 3.
4. **LCD Engine (Phase 3) Before RGB Engine (Phase 4):** Validates the high-bandwidth Interface A bulk pipe (`0xFF68`) before implementing matrix lighting and dual-tier flash persistence on Interface B (`0xFFFF`).

---

## Sources and Evidence Boundary

> **Reproducibility status:** The source paths below are present as working-tree artifacts, but no raw USB capture (`.pcap`/`.pcapng`) is included. The current `.gitignore` excludes the vendor and research directories, so a clean Git checkout cannot independently reproduce those inputs unless they are separately supplied or committed. Architecture decisions that depend on them remain provisional.

### Directly inspectable OEM metadata (HIGH for identity/layout only)
- `vendor_driver/device.xml`: `VID=05AC`, `PID=024F`, product name `Gaming Keyboard`, and OEM identifier `RKGK890`.
- `vendor_driver/KeyboardLayout.xml` and `research/layout_81keys.json`: OEM key definitions and the normalized 81-key layout.

### Target-board observations without raw capture (MEDIUM / reported)
- `research/protocol_notes.md` and `research/webhid_tester.html`: prose and tooling for reported WebHID/interface observations; neither is a raw USB capture.
- `research/capture_plan.md`: records unresolved Q1–Q4: LCD RAM versus Flash, feature-report interface, RGB persistence, and MCU ACK behavior.

### Prior art and cross-family inference (INFERRED; not verified on Monka)
- `research/prior_art_protocol.md` and `rcsn01/GMK-67-Driver`: RGB opcode sequences such as `04 13`, `04 18`, `04 20`, `04 02`, `04 F0`, and `04 F5` are prior art from GMK-67/related hardware, not Monka captures.
- `wsclx/ak820pro-modder` and `Aiacos/ajazz-control-center`: 128x128 RGB565, 4096-byte LCD chunking, ST7789-related behavior, and RTC `0x51` are Ajazz/Sonix-family references only.

### Architecture and platform references (MEDIUM)
- `crates.io` package registries, Apple IOKit/App Sandbox documentation, USB HID specifications, and OpenRGB provide implementation context and constraints; they do not prove the target-board protocol.

**Overall confidence:** MEDIUM for the architecture research; HIGH only for the OEM identity/layout metadata. Do not implement or whitelist target-board writes until raw captures and Q1–Q4 validation are available.

---
*Architecture research for: MonKey (MonkaKeyboard Rust Driver & Tooling Ecosystem)*
*Researched: 2026-09-13*
