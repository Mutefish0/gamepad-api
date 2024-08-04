import ffi from "./ffi.ts";

// TypeScript interfaces for Rust structs
interface Gamepad {
  index: number;
  axes: number[];
  buttons: Array<{ pressed: boolean; value: number }>;
}

const __ptr_gamepad_api__ = ffi.symbols.gamepad_api_new();

function getGamepads(): Gamepad[] {
  const bufPtr = ffi.symbols.get_gamepads(__ptr_gamepad_api__);

  const view = new DataView(bufPtr.buffer);

  const len = view.getBigUint64(0, true);
  const dataPtr = view.getBigUint64(8, true);
  const view2 = new Deno.UnsafePointerView(Deno.UnsafePointer.create(dataPtr)!);

  const gamepads: Gamepad[] = [];

  for (let i = 0; i < len; i++) {
    let offset = 0;

    const index = view2.getBigUint64(offset);
    offset += 8;
    const axes = [];
    for (let i = 0; i < 4; i++) {
      axes.push(view2.getFloat32(offset));
      offset += 4;
    }
    const buttons: Gamepad["buttons"] = [];
    for (let i = 0; i < 24; i++) {
      const pressed = view2.getUint8(offset) === 1;
      offset += 4;
      const value = view2.getFloat32(offset);
      offset += 4;
      buttons.push({ pressed, value });
    }
    gamepads.push({ index: Number(index), axes, buttons });
  }

  ffi.symbols.free_gamepad_array(bufPtr!);

  return gamepads;
}

export { getGamepads };
