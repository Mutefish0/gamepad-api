## `gamepad api`

The [Gamepad API](https://developer.mozilla.org/en-US/docs/Web/API/Gamepad) for Deno.

## Usage

```ts
import { getGamepads } from "jsr:@mutefish/gamepad-api";

async function main() {
  while (true) {
    const gamepads = getGamepads();
    console.log(gamepads);
    await new Promise((resolve) => setTimeout(resolve, 30));
  }
}

main();
```

## Support Contollers

- [x] Nintendo JoyCon
- [x] Nintendo Switch Pro
- [ ] Xbox 360
- [ ] PS5
