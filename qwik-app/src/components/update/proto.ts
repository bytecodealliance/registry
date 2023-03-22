import protobuf from "protobufjs"

export type Entry  = {
  algo: string
}
export const proto = async (code: string): Promise<{[k: string]: any} | undefined> => {
  return protobuf.load("./warg.proto").then(function (root) {
    const AwesomeMessage = root?.lookupType("warg.PackageRecord");

  // Exemplary payload
    const binary_string = window.atob(code);
    const len = binary_string.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binary_string.charCodeAt(i);
    }
    return AwesomeMessage.decode(bytes)
  })
};

