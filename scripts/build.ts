import { encodeBase64 } from "jsr:@std/encoding";

const targets = [
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
];

for (const target of targets) {
  const command = new Deno.Command("cross", {
    args: ["build", "--release", "--target", target],
    stdout: "inherit",
    stderr: "inherit",
  });

  const filename = target.includes("windows")
    ? "gamepad_api.dll"
    : target.includes("darwin")
    ? "libgamepad_api.dylib"
    : "libgamepad_api.so";

  const { code } = command.outputSync();

  if (code === 0) {
    const buf = Deno.readFileSync(`./target/${target}/release/${filename}`);
    const b64 = encodeBase64(buf);
    const code = `
      import { decodeBase64 } from "jsr:@std/encoding";
      export default decodeBase64("${b64}");
    `;
    Deno.writeTextFileSync(`./libs/${target}.ts`, code);
  }
}
