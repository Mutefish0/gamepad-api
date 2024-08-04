import "./mod.ts";

async function main() {
  while (true) {
    // @ts-ignore
    console.log(navigator.getGamepads());
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

main();
