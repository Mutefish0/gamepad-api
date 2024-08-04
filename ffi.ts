const suffix =
  Deno.build.os === "windows"
    ? ".dll"
    : Deno.build.os === "darwin"
    ? ".dylib"
    : ".so";

const buff = await import(`./libs/${Deno.build.target}.ts`);

const libPath = Deno.makeTempFileSync({ suffix });

Deno.writeFileSync(libPath, buff.default);

const lib = Deno.dlopen(libPath, {
  gamepad_api_new: { parameters: [], result: "pointer" },
  get_gamepads: {
    parameters: ["pointer"],
    result: { struct: ["usize", "pointer"] },
  },
  free_gamepad_array: {
    parameters: [{ struct: ["usize", "pointer"] }],
    result: "void",
  },
});

export default lib;
