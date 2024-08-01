use hidapi::{HidApi, HidDevice};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use zerocopy::*;

mod util;

const VENDOR_ID_NINTENDO: u16 = 0x057e;

const PRODUCT_ID_NINTENDO_JOYCONL: u16 = 0x2006;
const PRODUCT_ID_NINTENDO_JOYCONR: u16 = 0x2007;
const PRODUCT_IDNINTENDO_PROCON: u16 = 0x2009;

const DEVICE_TUPLES: [(u16, u16); 3] = [
    (VENDOR_ID_NINTENDO, PRODUCT_ID_NINTENDO_JOYCONL),
    (VENDOR_ID_NINTENDO, PRODUCT_ID_NINTENDO_JOYCONR),
    (VENDOR_ID_NINTENDO, PRODUCT_IDNINTENDO_PROCON),
];

#[repr(u8)]
enum ReportId {
    SubcmdReply = 0x21,
    IMU = 0x30,
    MCU = 0x31,
}

impl ReportId {
    fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0x21 => Some(ReportId::SubcmdReply),
            0x30 => Some(ReportId::IMU),
            0x31 => Some(ReportId::MCU),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct SubcmdReply {
    ack: u8,        /* MSB 1 for ACK, 0 for NACK */
    id: u8,         /* id of requested subcmd */
    data: [u8; 16], /* will be at most 35 bytes */
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct IMUData {
    accel_x: i16,
    accel_y: i16,
    accel_z: i16,
    gyro_x: i16,
    gyro_y: i16,
    gyro_z: i16,
}

enum SubData {
    Reply(SubcmdReply),
    IMU(IMUData),
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct JoyConReport {
    id: u8,
    timer: u8,
    bat_con: u8, /* battery and connection info */
    button_status: [u8; 3],
    left_stick: [u8; 3],
    right_stick: [u8; 3],
    vibrator_report: u8,
    sub_data: [u8; 18],
}

const REPORT_BUF_SIZE: usize = size_of::<JoyConReport>();

#[derive(Debug, Default)]
pub struct Gamepad {
    pub index: usize,
    pub axes: [f32; 4],
    pub buttons: [Button; 24],
}

#[derive(Debug, Default)]
pub struct Button {
    pub pressed: bool,
    pub value: f32,
}

pub struct GamepadAPI {
    hidapi: HidApi,
    device_map: HashMap<String, (usize, HidDevice)>,
    input_buf: [u8; REPORT_BUF_SIZE],
}

impl GamepadAPI {
    pub fn new() -> Self {
        let hidapi = HidApi::new().unwrap();
        GamepadAPI {
            hidapi,
            device_map: HashMap::new(),
            input_buf: [0; REPORT_BUF_SIZE],
        }
    }

    fn read_data_and_fill(buf: &mut [u8], device: &HidDevice, gamepad: &mut Gamepad) {
        let len = device.read(buf).unwrap();
        if len > 12 {
            if let Some(report_id) = ReportId::try_from_u8(buf[0]) {
                //const report = buf
                let report = JoyConReport::read_from(buf).unwrap();
                let button_values = util::extract_bits(&report.button_status, 3);
                for i in 0..24 {
                    gamepad.buttons[i].pressed = button_values[i] > 0;
                    gamepad.buttons[i].value = button_values[i] as f32;
                }
            }
        }
    }

    pub fn get_gamepads(&mut self) -> Vec<Gamepad> {
        let GamepadAPI {
            input_buf,
            device_map,
            hidapi,
        } = self;

        hidapi.reset_devices().unwrap();

        for (vid, pid) in DEVICE_TUPLES {
            hidapi.add_devices(vid, pid).unwrap();
        }

        let mut gamepads = Vec::new();

        let mut live_sns: HashSet<&str> = HashSet::new();

        for device_info in hidapi.device_list() {
            let sn = device_info.serial_number().unwrap();

            live_sns.insert(sn);

            let (index, device) = device_map.entry(sn.to_string()).or_insert_with(|| {
                let device = hidapi
                    .open_serial(device_info.vendor_id(), device_info.product_id(), sn)
                    .unwrap();

                (util::generate_id(), device)
            });

            let mut gamepad = Gamepad::default();

            gamepad.index = *index;
            Self::read_data_and_fill(input_buf, &device, &mut gamepad);

            gamepads.push(gamepad);
        }

        self.device_map
            .retain(|sn, _| live_sns.contains(sn.as_str()));

        gamepads
    }
}

// pub fn extract_bits(value: u8) -> [u8; 8] {
//     let mut bits = [0u8; 8];
//     for i in 0..8 {
//         let mask = 1 << i;
//         // Check if the bit is set and store 1 or 0 accordingly
//         bits[i] = if (value & mask) != 0 { 1 } else { 0 };
//     }
//     bits
// }

fn main() {}

// static int joycon_send_subcmd(struct joycon_ctlr *ctlr,
// 			      struct joycon_subcmd_request *subcmd,
// 			      size_t data_len, u32 timeout)
// {
// 	int ret;
// 	unsigned long flags;

// 	spin_lock_irqsave(&ctlr->lock, flags);
// 	/*
// 	 * If the controller has been removed, just return ENODEV so the LED
// 	 * subsystem doesn't print invalid errors on removal.
// 	 */
// 	if (ctlr->ctlr_state == JOYCON_CTLR_STATE_REMOVED) {
// 		spin_unlock_irqrestore(&ctlr->lock, flags);
// 		return -ENODEV;
// 	}
// 	memcpy(subcmd->rumble_data, ctlr->rumble_data[ctlr->rumble_queue_tail],
// 	       JC_RUMBLE_DATA_SIZE);
// 	spin_unlock_irqrestore(&ctlr->lock, flags);

// 	subcmd->output_id = JC_OUTPUT_RUMBLE_AND_SUBCMD;
// 	subcmd->packet_num = ctlr->subcmd_num;
// 	if (++ctlr->subcmd_num > 0xF)
// 		ctlr->subcmd_num = 0;
// 	ctlr->subcmd_ack_match = subcmd->subcmd_id;
// 	ctlr->msg_type = JOYCON_MSG_TYPE_SUBCMD;

// 	ret = joycon_hid_send_sync(ctlr, (u8 *)subcmd,
// 				   sizeof(*subcmd) + data_len, timeout);
// 	if (ret < 0)
// 		hid_dbg(ctlr->hdev, "send subcommand failed; ret=%d\n", ret);
// 	else
// 		ret = 0;
// 	return ret;
// }
