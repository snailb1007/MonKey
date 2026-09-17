pub mod bench;
pub mod device;
pub mod doctor;
pub mod error;
pub mod lcd;
pub mod protocol;
pub mod rgb;
pub mod transport;

pub use bench::{
    encode_frame, run_bulk_streaming_bench, run_bulk_streaming_bench_with_progress,
    run_transaction_latency_bench, run_transaction_latency_bench_with_progress,
    synthetic_lcd_frame, BenchmarkConfig, LatencyReport, ThroughputReport, TARGET_FPS_MAX,
    TARGET_FPS_MIN,
};
pub use device::{
    classify_interface, determine_capabilities, find_monka_device_sets, find_monka_devices,
    group_monka_devices, init_hidapi, open_device_path, parse_probe_response,
    DeviceDiagnostics, DiscoveredDevice, InterfaceCheckStatus, InterfacePolicy, InterfaceRole,
    MonkaDevice, MonkaDeviceSet, OpenError, ProbeOutput, MONKA_PID, MONKA_VID,
    PRODUCT_IDENTIFIER,
};
pub use doctor::{run_doctor_checks, CheckStatus, DiagnosticCheck, DoctorReport, DoctorSummary};
pub use error::{MonkeyError, TransportError};
pub use lcd::{
    convert_image_to_frame, decode_gif, decode_gif_frames, generate_test_pattern,
    generate_test_pattern_with_format, load_image, load_image_frame, preprocess_image,
    rgb565_bytes, rgb565_to_bytes, rgb888_to_rgb565, ColorFormat, FpsRegulator, FrameChunker,
    LcdAnimationFrame, LcdPacingConfig, LcdStreamMetrics, LcdStreamer, TestPatternType,
    LCD_CHUNK_COUNT, LCD_CHUNK_SIZE, LCD_FRAME_BYTES, LCD_HEIGHT, LCD_INTERFACE_A_REPORT_ID,
    LCD_PIXELS, LCD_WIDTH,
};
pub use protocol::channel::{HardwareChannel, HardwareCommand};
pub use protocol::crc::calculate_crc16;
pub use protocol::framing::{
    reassemble_chunks, slice_into_chunks, slice_lcd_frame, BulkTransferPlan, ChunkIterator,
};
pub use protocol::safety::{SafetyRails, WriteMode};
pub use protocol::transaction::TransactionManager;
pub use protocol::types::*;
pub use rgb::{
    decode_rgb_control_packet, encode_rgb_control_packet, FlowDirection, KeyDefinition, KeyMatrix,
    KeyPosition, LightingConfig, LightingMode, RgbColor, RgbManager, RgbProfile,
    CURRENT_PROFILE_SCHEMA_VERSION,
};
pub use transport::safe::SafeTransport;
pub use transport::{
    ConnectionState, HidTransport, MockTransport, SharedMockTransport, Transport, TransportCall,
};
