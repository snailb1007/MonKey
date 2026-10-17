//! macOS IOKit USB Control Request transport for Interface 3 (`bInterfaceNumber = 3`, `UsagePage 0xFF13`).
//!
//! On macOS, `AppleUserUSBHostHIDDevice` fails to attach to `IOUSBHostInterface@3` because its
//! interrupt pipe initialization returns `0xE00002F0`. As a result, `IOHIDManager` / `hidapi`
//! does not enumerate Interface 3, and sending 64-byte feature reports to Interface 1 (`0x000C:0x0001`)
//! fails with `0xE0005000 (pipe stalled)`.
//!
//! Because no kernel driver claims `IOUSBHostInterface@3`, userspace can open it directly via
//! `IOCreatePlugInInterfaceForService` (`kIOUSBInterfaceUserClientTypeID`) without root or TCC
//! permissions and issue standard USB HID Class Control Requests (`SET_REPORT` / `GET_REPORT`
//! on Endpoint 0 with `wIndex = 3`).

use crate::device::{MONKA_PID, MONKA_VID};
use crate::error::TransportError;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone)]
struct CFUUIDBytes {
    bytes: [u8; 16],
}

#[repr(C)]
struct IOUSBDevRequest {
    bm_request_type: u8,
    b_request: u8,
    w_value: u16,
    w_index: u16,
    w_length: u16,
    p_data: *mut c_void,
    w_len_done: u32,
}

type IoObjectT = u32;
type IoServiceT = IoObjectT;
type IoIteratorT = IoObjectT;
type KernReturnT = c_int;
type HResult = i32;

const KERN_SUCCESS: KernReturnT = 0;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_CF_NUMBER_SINT32_TYPE: c_int = 3;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingServices(
        main_port: u32,
        matching: *mut c_void,
        existing: *mut IoIteratorT,
    ) -> KernReturnT;
    fn IOIteratorNext(iterator: IoIteratorT) -> IoServiceT;
    fn IOObjectRelease(object: IoObjectT) -> KernReturnT;
    fn IORegistryEntryCreateCFProperty(
        entry: IoServiceT,
        key: *const c_void,
        allocator: *const c_void,
        options: u32,
    ) -> *mut c_void;
    fn IOCreatePlugInInterfaceForService(
        service: IoServiceT,
        plugin_type: *const c_void,
        interface_type: *const c_void,
        the_interface: *mut *mut *mut c_void,
        the_score: *mut i32,
    ) -> KernReturnT;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFUUIDGetConstantUUIDWithBytes(
        alloc: *const c_void,
        b0: u8,
        b1: u8,
        b2: u8,
        b3: u8,
        b4: u8,
        b5: u8,
        b6: u8,
        b7: u8,
        b8: u8,
        b9: u8,
        b10: u8,
        b11: u8,
        b12: u8,
        b13: u8,
        b14: u8,
        b15: u8,
    ) -> *const c_void;
    fn CFUUIDGetUUIDBytes(uuid: *const c_void) -> CFUUIDBytes;
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> *const c_void;
    fn CFNumberGetValue(number: *const c_void, the_type: c_int, value_ptr: *mut c_void) -> bool;
    fn CFRelease(cf: *const c_void);
}

unsafe fn get_int_property(service: IoServiceT, name: &[u8]) -> Option<i32> {
    let key = CFStringCreateWithCString(
        ptr::null(),
        name.as_ptr().cast::<c_char>(),
        K_CF_STRING_ENCODING_UTF8,
    );
    if key.is_null() {
        return None;
    }
    let val_ref = IORegistryEntryCreateCFProperty(service, key, ptr::null(), 0);
    CFRelease(key);
    if val_ref.is_null() {
        return None;
    }
    let mut val: i32 = 0;
    let ok = CFNumberGetValue(
        val_ref,
        K_CF_NUMBER_SINT32_TYPE,
        (&mut val as *mut i32).cast::<c_void>(),
    );
    CFRelease(val_ref);
    if ok {
        Some(val)
    } else {
        None
    }
}

/// Handle to `IOUSBHostInterface@3` (`bInterfaceNumber = 3`) on macOS for issuing
/// 64-byte HID Feature Report control transfers (`SET_REPORT` / `GET_REPORT`).
pub struct MacosUsbControlPipe {
    iface: *mut *mut c_void,
    interface_number: u16,
}

// Safety: `MacosUsbControlPipe` owns its IOKit COM interface pointer and is only accessed
// via `&mut self` on the single-flight hardware worker thread.
unsafe impl Send for MacosUsbControlPipe {}

impl MacosUsbControlPipe {
    /// Searches IORegistry for `IOUSBHostInterface` with `idVendor == MONKA_VID`,
    /// `idProduct == MONKA_PID`, and `bInterfaceNumber == 3`, and opens it.
    pub fn open() -> Result<Self, TransportError> {
        unsafe {
            let matching = IOServiceMatching(c"IOUSBHostInterface".as_ptr().cast::<c_char>());
            if matching.is_null() {
                return Err(TransportError::HidError(
                    "IOServiceMatching(IOUSBHostInterface) returned null".to_string(),
                ));
            }

            let mut iter: IoIteratorT = 0;
            let kr = IOServiceGetMatchingServices(0, matching, &mut iter);
            if kr != KERN_SUCCESS {
                return Err(TransportError::HidError(format!(
                    "IOServiceGetMatchingServices failed: 0x{:08X}",
                    kr as u32
                )));
            }

            let mut target_service: IoServiceT = 0;
            loop {
                let svc = IOIteratorNext(iter);
                if svc == 0 {
                    break;
                }
                let vid = get_int_property(svc, b"idVendor\0");
                let pid = get_int_property(svc, b"idProduct\0");
                let ifnum = get_int_property(svc, b"bInterfaceNumber\0");

                if vid == Some(MONKA_VID as i32)
                    && pid == Some(MONKA_PID as i32)
                    && ifnum == Some(3)
                {
                    target_service = svc;
                    break;
                }
                IOObjectRelease(svc);
            }
            IOObjectRelease(iter);

            if target_service == 0 {
                return Err(TransportError::DeviceNotFound(
                    "IOUSBHostInterface@3 (UsagePage 0xFF13) not found".to_string(),
                ));
            }

            // kIOUSBInterfaceUserClientTypeID: 2d9786c6-9ef3-11d4-ad51-000a27052861
            let plugin_type = CFUUIDGetConstantUUIDWithBytes(
                ptr::null(),
                0x2d,
                0x97,
                0x86,
                0xc6,
                0x9e,
                0xf3,
                0x11,
                0xd4,
                0xad,
                0x51,
                0x00,
                0x0a,
                0x27,
                0x05,
                0x28,
                0x61,
            );
            // kIOCFPlugInInterfaceID: C244E858-109C-11D4-91D4-0050E4C6426F
            let iocf_type = CFUUIDGetConstantUUIDWithBytes(
                ptr::null(),
                0xC2,
                0x44,
                0xE8,
                0x58,
                0x10,
                0x9C,
                0x11,
                0xD4,
                0x91,
                0xD4,
                0x00,
                0x50,
                0xE4,
                0xC6,
                0x42,
                0x6F,
            );
            // kIOUSBInterfaceInterfaceID182: 4923AC4C-4896-11D5-9208-000A27801E86
            let usb_iface_uuid = CFUUIDGetConstantUUIDWithBytes(
                ptr::null(),
                0x49,
                0x23,
                0xAC,
                0x4C,
                0x48,
                0x96,
                0x11,
                0xD5,
                0x92,
                0x08,
                0x00,
                0x0A,
                0x27,
                0x80,
                0x1E,
                0x86,
            );

            let mut plugin: *mut *mut c_void = ptr::null_mut();
            let mut score: i32 = 0;
            let kr = IOCreatePlugInInterfaceForService(
                target_service,
                plugin_type,
                iocf_type,
                &mut plugin,
                &mut score,
            );
            IOObjectRelease(target_service);

            if kr != KERN_SUCCESS || plugin.is_null() {
                return Err(TransportError::HidError(format!(
                    "IOCreatePlugInInterfaceForService failed: 0x{:08X}",
                    kr as u32
                )));
            }

            let plugin_vtable = (*plugin).cast::<*const c_void>();
            // Index 1: QueryInterface, Index 3: Release
            let query_interface: unsafe extern "C" fn(
                *mut *mut c_void,
                CFUUIDBytes,
                *mut *mut *mut c_void,
            ) -> HResult = std::mem::transmute(*plugin_vtable.add(1));
            let release_plugin: unsafe extern "C" fn(*mut *mut c_void) -> u32 =
                std::mem::transmute(*plugin_vtable.add(3));

            let mut iface: *mut *mut c_void = ptr::null_mut();
            let hr = query_interface(plugin, CFUUIDGetUUIDBytes(usb_iface_uuid), &mut iface);
            release_plugin(plugin);

            if hr != 0 || iface.is_null() {
                return Err(TransportError::HidError(format!(
                    "QueryInterface(IOUSBInterfaceInterface182) failed: 0x{:08X}",
                    hr as u32
                )));
            }

            let iface_vtable = (*iface).cast::<*const c_void>();
            // Index 8: USBInterfaceOpen, Index 3: Release
            let usb_open: unsafe extern "C" fn(*mut *mut c_void) -> KernReturnT =
                std::mem::transmute(*iface_vtable.add(8));
            let release_iface: unsafe extern "C" fn(*mut *mut c_void) -> u32 =
                std::mem::transmute(*iface_vtable.add(3));

            let kr = usb_open(iface);
            if kr != KERN_SUCCESS {
                release_iface(iface);
                return Err(TransportError::HidError(format!(
                    "USBInterfaceOpen(Interface 3) failed: 0x{:08X}",
                    kr as u32
                )));
            }

            Ok(Self {
                iface,
                interface_number: 3,
            })
        }
    }

    /// Sends a 64-byte HID Feature Report (`SET_REPORT`, Report Type 3, Report ID 0) over Endpoint 0.
    pub fn send_feature_report(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let payload = if data.len() == 65 && data[0] == 0 {
            &data[1..65]
        } else if data.len() >= 64 {
            &data[..64]
        } else {
            return Err(TransportError::BufferTooSmall {
                needed: 64,
                provided: data.len(),
            });
        };

        let mut buf = [0u8; 64];
        buf.copy_from_slice(payload);

        let mut req = IOUSBDevRequest {
            bm_request_type: 0x21, // Host-to-Device, Class, Interface
            b_request: 0x09,       // HID SET_REPORT
            w_value: 0x0300,       // Report Type 3 (Feature), Report ID 0
            w_index: self.interface_number,
            w_length: 64,
            p_data: buf.as_mut_ptr().cast::<c_void>(),
            w_len_done: 0,
        };

        unsafe {
            let iface_vtable = (*self.iface).cast::<*const c_void>();
            // Index 24: ControlRequest(self, pipeRef=0, &req)
            let control_request: unsafe extern "C" fn(
                *mut *mut c_void,
                u8,
                *mut IOUSBDevRequest,
            ) -> KernReturnT = std::mem::transmute(*iface_vtable.add(24));

            let kr = control_request(self.iface, 0, &mut req);
            if kr != KERN_SUCCESS {
                return Err(TransportError::HidError(format!(
                    "ControlRequest(SET_REPORT) on Interface 3 failed: 0x{:08X}",
                    kr as u32
                )));
            }
        }
        Ok(())
    }

    /// Reads a 64-byte HID Feature Report (`GET_REPORT`, Report Type 3, Report ID 0) over Endpoint 0.
    pub fn get_feature_report(
        &mut self,
        report_id: u8,
        buf: &mut [u8],
    ) -> Result<usize, TransportError> {
        if buf.len() < 64 {
            return Err(TransportError::BufferTooSmall {
                needed: 64,
                provided: buf.len(),
            });
        }

        let mut raw = [0u8; 64];
        let mut req = IOUSBDevRequest {
            bm_request_type: 0xA1, // Device-to-Host, Class, Interface
            b_request: 0x01,       // HID GET_REPORT
            w_value: 0x0300,       // Report Type 3 (Feature), Report ID 0
            w_index: self.interface_number,
            w_length: 64,
            p_data: raw.as_mut_ptr().cast::<c_void>(),
            w_len_done: 0,
        };

        unsafe {
            let iface_vtable = (*self.iface).cast::<*const c_void>();
            let control_request: unsafe extern "C" fn(
                *mut *mut c_void,
                u8,
                *mut IOUSBDevRequest,
            ) -> KernReturnT = std::mem::transmute(*iface_vtable.add(24));

            let kr = control_request(self.iface, 0, &mut req);
            if kr != KERN_SUCCESS {
                return Err(TransportError::HidError(format!(
                    "ControlRequest(GET_REPORT) on Interface 3 failed: 0x{:08X}",
                    kr as u32
                )));
            }
        }

        let read_len = (req.w_len_done as usize).min(64);
        if buf.len() >= 65 && report_id == 0 {
            buf[0] = 0;
            buf[1..1 + read_len].copy_from_slice(&raw[..read_len]);
            Ok(1 + read_len)
        } else {
            buf[..read_len].copy_from_slice(&raw[..read_len]);
            Ok(read_len)
        }
    }
}

impl Drop for MacosUsbControlPipe {
    fn drop(&mut self) {
        if !self.iface.is_null() {
            unsafe {
                let iface_vtable = (*self.iface).cast::<*const c_void>();
                // Index 9: USBInterfaceClose, Index 3: Release
                let usb_close: unsafe extern "C" fn(*mut *mut c_void) -> KernReturnT =
                    std::mem::transmute(*iface_vtable.add(9));
                let release_iface: unsafe extern "C" fn(*mut *mut c_void) -> u32 =
                    std::mem::transmute(*iface_vtable.add(3));
                let _ = usb_close(self.iface);
                let _ = release_iface(self.iface);
            }
            self.iface = ptr::null_mut();
        }
    }
}
