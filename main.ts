import { getGamepads } from "./mod.ts";

async function main() {
  while (true) {
    const gamepads = getGamepads();
    console.log(gamepads);
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

main();
