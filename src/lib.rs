/**
 * port form:
 *  https://source.chromium.org/chromium/chromium/src/+/main:device/gamepad/nintendo_controller.cc;bpv=0;bpt=1
 *  https://github.com/libsdl-org/SDL/blob/efefc4a1f35812007663f4afccd7bae68496238f/src/joystick/hidapi/SDL_hidapi_switch.c#L87
 */
use hidapi::{HidApi, HidDevice};
use num_enum::TryFromPrimitive;
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

// Bogus calibration value that should be ignored.
const CAL_BOGUS_VALUE: u16 = 0xfff;
// Default calibration values to use if the controller returns bogus values.
const CAL_DEFAULT_DEADZONE: u16 = 160;
const CAL_DEFAULT_MIN: u16 = 550;
const CAL_DEFAULT_CENTER: u16 = 2050;
const CAL_DEFAULT_MAX: u16 = 3550;

#[repr(u8)]
enum OutputReportID {
    RumbleAndSubcommand = 0x01,
    RumbleOnly = 0x10,
    Proprietary = 0x80,
}

#[repr(u8)]
#[derive(Debug, TryFromPrimitive)]
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

#[repr(u16)]
#[derive(Debug, TryFromPrimitive)]
enum SPIAddress {
    // SPI memory regions.
    ImuCalibration = 0x6020,
    //const size_t kSpiImuCalibrationSize = 24;
    AnalogStickCalibration = 0x603d,
    //const size_t kSpiAnalogStickCalibrationSize = 18;
    ImuHorizontalOffsets = 0x6080,
    //const size_t kSpiImuHorizontalOffsetsSize = 6;
    AnalogStickParameters = 0x6086,
    //const size_t kSpiAnalogStickParametersSize = 18;
}

#[repr(u8)]
#[derive(Debug, TryFromPrimitive)]
enum InputReportID {
    SubcommandReply = 0x21,
    FullControllerState = 0x30,
    FullControllerAndMcuState = 0x31,
    SimpleControllerState = 0x3F,
    CommandAck = 0x81,
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
struct SimpleControllerStatePacket {
    button_status: [u8; 3],
    left_stick: [u8; 3],
    right_stick: [u8; 3],
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct ControllerStatePacket {
    counter: u8,
    battery_and_connection: u8, /* battery and connection info */
    simple_state: SimpleControllerStatePacket,
    vibration_code: u8,
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default)]
struct SubcommandInputPacket {
    controller_state: ControllerStatePacket,
    subcommand_ack: u8,
    subcommand_id: u8,
    address: u16,
    padding: [u8; 2], // 0x00 0x00
    length: u8,
    subcommand_data: [u8; 18],
}

const REPORT_BUF_SIZE: usize = 48;

#[repr(C)]
#[derive(FromBytes, FromZeroes, Default, Debug)]
struct AnalogStickCalibrationPacket {
    // Analog stick calibration data.
    l_xy_max: [u8; 3],
    l_xy_center: [u8; 3],
    l_xy_min: [u8; 3],

    r_xy_center: [u8; 3],
    r_xy_min: [u8; 3],
    r_xy_max: [u8; 3],
}
#[repr(C)]
#[derive(FromBytes, FromZeroes, Default, Debug)]
struct AnalogStickParamsCalibrationPacket {
    // Analog stick parameters
    padding: [u8; 3],

    // dead zone and range ratio
    params: [u8; 3],
}

#[derive(Debug)]
struct CalibrationData {
    // Analog stick calibration data.
    lx_center: u16,
    lx_min: u16,
    lx_max: u16,
    ly_center: u16,
    ly_min: u16,
    ly_max: u16,
    rx_center: u16,
    rx_min: u16,
    rx_max: u16,
    ry_center: u16,
    ry_min: u16,
    ry_max: u16,

    dead_zone: u16,
    range_ratio: u16,

    // IMU calibration data.
    accelerometer_origin_x: u16,
    accelerometer_origin_y: u16,
    accelerometer_origin_z: u16,
    accelerometer_sensitivity_x: u16,
    accelerometer_sensitivity_y: u16,
    accelerometer_sensitivity_z: u16,
    gyro_origin_x: u16,
    gyro_origin_y: u16,
    gyro_origin_z: u16,
    gyro_sensitivity_x: u16,
    gyro_sensitivity_y: u16,
    gyro_sensitivity_z: u16,
    horizontal_offset_x: u16,
    horizontal_offset_y: u16,
    horizontal_offset_z: u16,
}

impl Default for CalibrationData {
    fn default() -> Self {
        CalibrationData {
            lx_center: CAL_DEFAULT_CENTER,
            lx_min: CAL_DEFAULT_MIN,
            lx_max: CAL_DEFAULT_MAX,
            ly_center: CAL_DEFAULT_CENTER,
            ly_min: CAL_DEFAULT_MIN,
            ly_max: CAL_DEFAULT_MAX,
            rx_center: CAL_DEFAULT_CENTER,
            rx_min: CAL_DEFAULT_MIN,
            rx_max: CAL_DEFAULT_MAX,
            ry_center: CAL_DEFAULT_CENTER,
            ry_min: CAL_DEFAULT_MIN,
            ry_max: CAL_DEFAULT_MAX,

            dead_zone: CAL_DEFAULT_DEADZONE,
            range_ratio: 0,

            // IMU calibration data.
            accelerometer_origin_x: 0,
            accelerometer_origin_y: 0,
            accelerometer_origin_z: 0,
            accelerometer_sensitivity_x: 0,
            accelerometer_sensitivity_y: 0,
            accelerometer_sensitivity_z: 0,
            gyro_origin_x: 0,
            gyro_origin_y: 0,
            gyro_origin_z: 0,
            gyro_sensitivity_x: 0,
            gyro_sensitivity_y: 0,
            gyro_sensitivity_z: 0,
            horizontal_offset_x: 0,
            horizontal_offset_y: 0,
            horizontal_offset_z: 0,
        }
    }
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, AsBytes, Default)]
struct SubcommandOutputPacket {
    report_id: u8,
    report_counter: u8,
    rumble_data: [u8; 8],
    subcommand_id: u8,
    subcommand_data: [u8; 32], // 38
}

#[repr(C)]
#[derive(FromBytes, FromZeroes, AsBytes, Default)]
struct ReadSpiPacket {
    address: u16,
    padding: u16,
    length: u16,
}

#[repr(C)]
#[derive(Default)]
pub struct Gamepad {
    pub index: usize,
    pub axes: [f32; 4],
    pub buttons: [Button; 24],
}

#[repr(C)]
pub struct GamepadArray {
    len: usize,
    data: *const Gamepad,
}

#[repr(C)]
#[derive(Default)]
pub struct Button {
    pub pressed: bool,
    _padding: [u8; 3],
    pub value: f32,
}

#[derive(Debug, Default)]
struct GamepadContext {
    cal_data: CalibrationData,
    init_state: GamepadInitState,
}

#[derive(Debug, Default, TryFromPrimitive, Clone, Copy)]
#[repr(u8)]
enum GamepadInitState {
    #[default]
    Uninitialized = 0,
    RequestedAnalogCalibration = 1,
    RequestedAnalogCalibrationOk = 2,
    RequestedAnalogParams = 3,
    RequestedAnalogParamsOk = 4,
    Initialized = 5,
}

pub struct GamepadAPI {
    hidapi: HidApi,
    device_map: HashMap<String, (usize, HidDevice, GamepadContext)>,
    input_buf: [u8; REPORT_BUF_SIZE],
    report_counter: u8,
}

impl GamepadAPI {
    pub fn new() -> Self {
        let hidapi = HidApi::new().unwrap();
        GamepadAPI {
            hidapi,
            device_map: HashMap::new(),
            input_buf: [0; REPORT_BUF_SIZE],
            report_counter: 0,
        }
    }

    fn send_subcommand(
        report_counter: &mut u8,
        device: &HidDevice,
        command_id: SubcommandID,
        buf: &[u8],
    ) {
        let mut packet = SubcommandOutputPacket::default();
        packet.report_id = OutputReportID::RumbleAndSubcommand as u8;
        packet.subcommand_id = command_id as u8;
        *report_counter = (*report_counter + 1) & 0xF;
        packet.report_counter = *report_counter;
        packet.subcommand_data[..buf.len()].copy_from_slice(buf);
        device.write(&packet.as_bytes()).unwrap();
    }

    fn request_analog_calibration(report_counter: &mut u8, device: &HidDevice) {
        let packet = ReadSpiPacket {
            address: SPIAddress::AnalogStickCalibration as u16,
            padding: 0_u16,
            length: 18_u16,
        };
        Self::send_subcommand(
            report_counter,
            device,
            SubcommandID::SPIFlashRead,
            packet.as_bytes(),
        );
    }

    fn request_analog_params(report_counter: &mut u8, device: &HidDevice) {
        let packet = ReadSpiPacket {
            address: SPIAddress::AnalogStickParameters as u16,
            padding: 0_u16,
            length: 18_u16,
        };
        Self::send_subcommand(
            report_counter,
            device,
            SubcommandID::SPIFlashRead,
            packet.as_bytes(),
        );
    }

    fn update_gamepad(
        state: &SimpleControllerStatePacket,
        cal_data: &CalibrationData,
        gamepad: &mut Gamepad,
    ) {
        let button_values = util::extract_bits(&state.button_status, 3);
        for i in 0..24 {
            gamepad.buttons[i].pressed = button_values[i] > 0;
            gamepad.buttons[i].value = button_values[i] as f32;
        }

        let mut lx: u16 = 0;
        let mut ly: u16 = 0;
        let mut rx: u16 = 0;
        let mut ry: u16 = 0;

        util::unpack_shorts(&state.left_stick, &mut lx, &mut ly);
        util::unpack_shorts(&state.right_stick, &mut rx, &mut ry);

        let is_left_deadzone = util::is_dead_zone(
            lx,
            ly,
            cal_data.lx_center,
            cal_data.ly_center,
            cal_data.dead_zone,
        );

        let is_right_deadzone = util::is_dead_zone(
            rx,
            ry,
            cal_data.rx_center,
            cal_data.ry_center,
            cal_data.dead_zone,
        );

        gamepad.axes[0] = if is_left_deadzone {
            0.0
        } else {
            util::clamp_axis(lx, cal_data.lx_min, cal_data.lx_max)
        };
        gamepad.axes[1] = if is_left_deadzone {
            0.0
        } else {
            util::clamp_axis(ly, cal_data.ly_min, cal_data.ly_max)
        };
        gamepad.axes[2] = if is_right_deadzone {
            0.0
        } else {
            util::clamp_axis(rx, cal_data.rx_min, cal_data.rx_max)
        };
        gamepad.axes[3] = if is_right_deadzone {
            0.0
        } else {
            util::clamp_axis(ry, cal_data.ry_min, cal_data.ry_max)
        };
    }

    fn update_stick_calibration_data(
        cal: &AnalogStickCalibrationPacket,
        cal_data: &mut CalibrationData,
    ) {
        util::unpack_shorts(
            &cal.l_xy_center,
            &mut cal_data.lx_center,
            &mut cal_data.ly_center,
        );
        util::unpack_shorts(
            &cal.r_xy_center,
            &mut cal_data.rx_center,
            &mut cal_data.ry_center,
        );
        util::unpack_shorts(&cal.l_xy_min, &mut cal_data.lx_min, &mut cal_data.ly_min);
        util::unpack_shorts(&cal.r_xy_min, &mut cal_data.rx_min, &mut cal_data.ry_min);

        util::unpack_shorts(&cal.l_xy_max, &mut cal_data.lx_max, &mut cal_data.ly_max);
        util::unpack_shorts(&cal.r_xy_max, &mut cal_data.rx_max, &mut cal_data.ry_max);
        if cal_data.lx_min == CAL_BOGUS_VALUE && cal_data.ly_max == CAL_BOGUS_VALUE {
            cal_data.lx_min = CAL_DEFAULT_MIN;
            cal_data.lx_max = CAL_DEFAULT_MAX;
            cal_data.lx_center = CAL_DEFAULT_CENTER;
            cal_data.ly_min = CAL_DEFAULT_MIN;
            cal_data.ly_max = CAL_DEFAULT_MAX;
            cal_data.ly_center = CAL_DEFAULT_CENTER;
        } else {
            cal_data.lx_min = cal_data.lx_center - cal_data.lx_min;
            cal_data.lx_max = cal_data.lx_center + cal_data.lx_max;
            cal_data.ly_min = cal_data.ly_center - cal_data.ly_min;
            cal_data.ly_max = cal_data.ly_center + cal_data.ly_max;
        }

        if cal_data.rx_min == CAL_BOGUS_VALUE && cal_data.ry_max == CAL_BOGUS_VALUE {
            cal_data.rx_min = CAL_DEFAULT_MIN;
            cal_data.rx_max = CAL_DEFAULT_MAX;
            cal_data.rx_center = CAL_DEFAULT_CENTER;
            cal_data.ry_min = CAL_DEFAULT_MIN;
            cal_data.ry_max = CAL_DEFAULT_MAX;
            cal_data.ry_center = CAL_DEFAULT_CENTER;
        } else {
            cal_data.rx_min = cal_data.rx_center - cal_data.rx_min;
            cal_data.rx_max = cal_data.rx_center + cal_data.rx_max;
            cal_data.ry_min = cal_data.ry_center - cal_data.ry_min;
            cal_data.ry_max = cal_data.ry_center + cal_data.ry_max;
        }
    }

    fn update_stick_params_calibration_data(
        cal: &AnalogStickParamsCalibrationPacket,
        cal_data: &mut CalibrationData,
    ) {
        util::unpack_shorts(
            &cal.params,
            &mut cal_data.dead_zone,
            &mut cal_data.range_ratio,
        );
        if cal_data.dead_zone == CAL_BOGUS_VALUE {
            cal_data.dead_zone = CAL_DEFAULT_DEADZONE;
        }
    }

    fn read_data_and_fill(
        device: &HidDevice,
        gamepad: &mut Gamepad,
        cal_data: &mut CalibrationData,
        init_state: &mut GamepadInitState,
        buf: &mut [u8],
    ) {
        let len = device.read(buf).unwrap();

        if len >= 12 {
            match InputReportID::try_from(buf[0]) {
                Ok(InputReportID::FullControllerState) => {
                    let report = ControllerStatePacket::read_from_prefix(&buf[1..]).unwrap();
                    Self::update_gamepad(&report.simple_state, &cal_data, gamepad);
                }
                Ok(InputReportID::SimpleControllerState) => {
                    let state = SimpleControllerStatePacket::read_from_prefix(&buf[1..]).unwrap();
                    Self::update_gamepad(&state, &cal_data, gamepad);
                }
                Ok(InputReportID::SubcommandReply) => {
                    let pack = SubcommandInputPacket::read_from_prefix(&buf[1..]).unwrap();
                    Self::update_gamepad(&pack.controller_state.simple_state, &cal_data, gamepad);
                    match SubcommandID::try_from(pack.subcommand_id) {
                        Ok(SubcommandID::SPIFlashRead) => {
                            match SPIAddress::try_from(pack.address) {
                                Ok(SPIAddress::AnalogStickCalibration) => {
                                    let cal = AnalogStickCalibrationPacket::read_from_prefix(
                                        &pack.subcommand_data,
                                    )
                                    .unwrap();

                                    Self::update_stick_calibration_data(&cal, cal_data);

                                    *init_state = GamepadInitState::RequestedAnalogCalibrationOk;
                                }
                                Ok(SPIAddress::AnalogStickParameters) => {
                                    let cal = AnalogStickParamsCalibrationPacket::read_from_prefix(
                                        &pack.subcommand_data,
                                    )
                                    .unwrap();
                                    Self::update_stick_params_calibration_data(&cal, cal_data);

                                    *init_state = GamepadInitState::RequestedAnalogParamsOk;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

impl GamepadAPI {
    pub fn get_gamepads(&mut self) -> Vec<Gamepad> {
        let GamepadAPI {
            input_buf,
            device_map,
            hidapi,
            report_counter,
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

            let (index, device, context) = device_map.entry(sn.to_string()).or_insert_with(|| {
                let device = hidapi
                    .open_serial(device_info.vendor_id(), device_info.product_id(), sn)
                    .unwrap();
                (util::generate_id(), device, GamepadContext::default())
            });

            let GamepadContext {
                cal_data,
                init_state,
            } = context;

            let mut gamepad = Gamepad::default();

            gamepad.index = *index;

            Self::read_data_and_fill(&device, &mut gamepad, cal_data, init_state, input_buf);

            match GamepadInitState::try_from(*init_state) {
                Ok(GamepadInitState::Uninitialized) => {
                    Self::request_analog_calibration(report_counter, &device);
                    context.init_state = GamepadInitState::RequestedAnalogCalibration;
                }
                Ok(GamepadInitState::RequestedAnalogCalibrationOk) => {
                    Self::request_analog_params(report_counter, &device);
                    context.init_state = GamepadInitState::RequestedAnalogParams;
                }
                Ok(GamepadInitState::RequestedAnalogParamsOk) => {
                    context.init_state = GamepadInitState::Initialized;
                }
                _ => {}
            }

            gamepads.push(gamepad);
        }

        self.device_map
            .retain(|sn, _| live_sns.contains(sn.as_str()));

        gamepads
    }
}

#[no_mangle]
pub extern "C" fn gamepad_api_new() -> *mut GamepadAPI {
    Box::into_raw(Box::new(GamepadAPI::new()))
}

#[no_mangle]
pub extern "C" fn get_gamepads(api: *mut GamepadAPI) -> GamepadArray {
    unsafe {
        let api = api.as_mut().unwrap();
        let gamepads = api.get_gamepads();
        let len = gamepads.len();
        let data = Box::into_raw(gamepads.into_boxed_slice()) as *const Gamepad;
        GamepadArray { data, len }
    }
}

#[no_mangle]
pub extern "C" fn free_gamepad_array(array: GamepadArray) {
    if !array.data.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(array.data as *mut Gamepad, array.len, array.len);
        }
    }
}
