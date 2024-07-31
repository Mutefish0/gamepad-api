extern crate hidapi;
use hidapi::{HidApi, HidDevice};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

const VENDOR_ID_NINTENDO: u16 = 0x057e;

const PRODUCT_ID_NINTENDO_JOYCONL: u16 = 0x2006;
const PRODUCT_ID_NINTENDO_JOYCONR: u16 = 0x2007;
const PRODUCT_IDNINTENDO_PROCON: u16 = 0x2009;

const DEVICE_TUPLES: [(u16, u16); 3] = [
    (VENDOR_ID_NINTENDO, PRODUCT_ID_NINTENDO_JOYCONL),
    (VENDOR_ID_NINTENDO, PRODUCT_ID_NINTENDO_JOYCONR),
    (VENDOR_ID_NINTENDO, PRODUCT_IDNINTENDO_PROCON),
];

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(0);

fn generate_id() -> usize {
    GLOBAL_ID.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Default)]
struct Gamepad {
    pub index: usize,
    pub axes: [f32; 4],
}

// // 泛型函数，接收 HashMap、键、以及一个闭包（用于生成默认值）
// fn get_or_insert<F>(map: &mut HashMap<String, String>, key: String, default_value: F) -> String
// where
//     F: FnOnce() -> String,
// {
//     // 使用 entry API 检查键是否存在
//     map.entry(key.clone())
//         .or_insert_with(default_value) // 如果键不存在，则插入默认值
//         .clone() // 返回键对应的值
// }

struct GamepadAPI {
    hidapi: HidApi,
    device_map: HashMap<String, (usize, HidDevice)>,
}

impl GamepadAPI {
    pub fn new() -> Self {
        let hidapi = hidapi::HidApi::new().unwrap();
        GamepadAPI {
            hidapi,
            device_map: HashMap::new(),
        }
    }

    // fn get_or_insert_gamepad(&mut self, sn: &str) -> &Gamepad {
    //     if !self.gamepads.contains_key(sn) {
    //         let new_gamepad = Gamepad::new(sn.to_string());
    //         self.gamepads.insert(sn.to_string(), new_gamepad);
    //     }
    //     self.gamepads.get(sn).unwrap()
    // }

    pub fn get_gamepads(&mut self) -> Vec<Gamepad> {
        self.hidapi.reset_devices().unwrap();

        for (vid, pid) in DEVICE_TUPLES {
            self.hidapi.add_devices(vid, pid).unwrap();
        }

        let mut gamepads = Vec::new();

        let mut live_sns: Vec<&str> = vec![];

        // let mut all_sns: HashSet<&str> = HashSet::new();
        // let mut new_sns: Vec<&str> = Vec::new();

        for device_info in self.hidapi.device_list() {
            let sn = device_info.serial_number().unwrap();

            live_sns.push(sn);

            let (index, device) = self.device_map.entry(sn.to_string()).or_insert_with(|| {
                let device = self
                    .hidapi
                    .open_serial(device_info.vendor_id(), device_info.product_id(), sn)
                    .unwrap();

                (generate_id(), device)
            });

            let mut gamepad = Gamepad::default();

            gamepad.index = *index;

            // device.get_device_info();

            // device.read(buf)

            gamepads.push(gamepad);
        }

        // delete offline devices

        gamepads
    }
}

fn main() {
    // let api = hidapi::HidApi::new().unwrap();
    // // Print out information about all connected devices
    // for device in api.device_list() {
    //     println!("{:#?}", device);
    // }

    // // Connect to device using its VID and PID
    // //let (VID, PID) = (0x0123, 0x3456);

    // let device = api.open(VID, PID).unwrap();

    // // Read data from device
    // let mut buf = [0u8; 8];
    // let res = device.read(&mut buf[..]).unwrap();
    // println!("Read: {:?}", &buf[..res]);

    // // Write data to device
    // let buf = [0u8, 1, 2, 3, 4];
    // let res = device.write(&buf).unwrap();
    // println!("Wrote: {:?} byte(s)", res);
}
