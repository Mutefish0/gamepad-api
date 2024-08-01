use hidapi::{HidApi, HidDevice};
use std::collections::{HashMap, HashSet};
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
enum OutputReportID {
    RumbleAndSubcommand = 0x01,
    RumbleOnly = 0x10,
    Proprietary = 0x80,
}

#[repr(u8)]
enum SubcommandID {
    BluetoothManualPair = 0x01,
    RequestDeviceInfo = 0x02,
    SetInputReportMode = 0x03,
    SetHCIState = 0x06,
    SPIFlashRead = 0x10,
    SetPlayerLights = 0x30,
    SetHomeLight = 0x38,
    EnableIMU = 0x40,
    SetIMUSensitivity = 0x41,
    EnableVibration = 0x48,
}

#[repr(u8)]
enum InputReportID {
    SubcommandReply = 0x21,
    FullControllerState = 0x30,
    FullControllerAndMcuState = 0x31,
    SimpleControllerState = 0x3F,
    CommandAck = 0x81,
}

impl InputReportID {
    fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0x21 => Some(InputReportID::SubcommandReply),
            0x30 => Some(InputReportID::FullControllerState),
            0x31 => Some(InputReportID::FullControllerAndMcuState),
            0x3F => Some(InputReportID::SimpleControllerState),
            0x81 => Some(InputReportID::CommandAck),
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

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct ControllerStatePacket {
    counter: u8,
    battery_and_connection: u8, /* battery and connection info */
    button_status: [u8; 3],
    left_stick: [u8; 3],
    right_stick: [u8; 3],
    vibration_code: u8,
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct SubcommandInputPacket {
    controller_state: ControllerStatePacket,
    subcommand_ack: u8,
    subcommand_id: u8,
    subcommand_data: [u8; 32],
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct SimpleControllerStatePacket {
    button_status: [u8; 3],
    left_stick: [u8; 3],
    right_stick: [u8; 3],
}

struct JobConRequest {
    subcmd_id: u8,
    rumble_data: [u8; 8],
    data: [u8; 0],
}

const REPORT_BUF_SIZE: usize = 48;

fn to_axis(value: u32) -> f32 {
    value as f32
}

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

    fn send_subcommand(device: &HidDevice, commandId: SubcommandID, buf: &[u8], len: usize) {
        //
    }

    fn read_data_and_fill(device: &HidDevice, gamepad: &mut Gamepad, buf: &mut [u8]) {
        let len = device.read(buf).unwrap();

        //println!("raw len: {}", len);

        if len >= 12 {
            let hex_string: String = buf.iter().map(|byte| format!("0x{:02x} ", byte)).collect();
            //println!("raw: {}", hex_string);

            match InputReportID::try_from_u8(buf[0]) {
                Some(InputReportID::FullControllerState) => {
                    let report = ControllerStatePacket::read_from_prefix(&buf[1..]).unwrap();
                    let button_values = util::extract_bits(&report.button_status, 3);

                    for i in 0..24 {
                        gamepad.buttons[i].pressed = button_values[i] > 0;
                        gamepad.buttons[i].value = button_values[i] as f32;
                    }

                    let left_x: u32 = ((report.left_stick[1] & 0x0F) as u32) << 4
                        | (report.left_stick[1] as u32) >> 4;
                    let left_y: u32 = report.left_stick[2] as u32;

                    let right_x: u32 = ((report.right_stick[1] & 0x0F) as u32) << 4
                        | (report.right_stick[1] as u32) >> 4;
                    let right_y: u32 = report.right_stick[2] as u32;

                    gamepad.axes[0] = to_axis(left_x);
                    gamepad.axes[1] = to_axis(left_y);
                    gamepad.axes[2] = to_axis(right_x);
                    gamepad.axes[3] = to_axis(right_y);
                }
                Some(InputReportID::SimpleControllerState) => {
                    let report = SimpleControllerStatePacket::read_from_prefix(&buf[1..]).unwrap();

                    let button_values = util::extract_bits(&report.button_status, 3);
                    for i in 0..24 {
                        gamepad.buttons[i].pressed = button_values[i] > 0;
                        gamepad.buttons[i].value = button_values[i] as f32;
                    }

                    let left_x: u32 =
                        (report.left_stick[0] as u32) << 4 | (report.left_stick[1] as u32) >> 4;
                    let left_y: u32 =
                        ((report.left_stick[1] as u32) & 0x0F) << 8 | report.left_stick[2] as u32;

                    let right_x: u32 =
                        (report.right_stick[0] as u32) << 4 | (report.right_stick[1] as u32) >> 4;
                    let right_y: u32 =
                        ((report.right_stick[1] as u32) & 0x0F) << 8 | report.right_stick[2] as u32;

                    gamepad.axes[0] = to_axis(left_x);
                    gamepad.axes[1] = to_axis(left_y);
                    gamepad.axes[2] = to_axis(right_x);
                    gamepad.axes[3] = to_axis(right_y);
                }
                _ => {}
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
            Self::read_data_and_fill(&device, &mut gamepad, input_buf);

            gamepads.push(gamepad);
        }

        self.device_map
            .retain(|sn, _| live_sns.contains(sn.as_str()));

        gamepads
    }
}

fn main() {
    let mut gamepad_api = GamepadAPI::new();
    loop {
        let gamepads = gamepad_api.get_gamepads();
        for gamepad in gamepads {
            println!("axis: {:?}", gamepad.axes);
            println!("buttons: {:?}", &gamepad.buttons[0..4]);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
