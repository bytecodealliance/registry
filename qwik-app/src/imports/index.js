import fs from "fs"

const enc = new TextEncoder()
const dec = new TextDecoder()
export function storeRegistryInfo() {
  fs.writeFileSync("./storage.json", "{\"registry\": \"http://127.0.0.1:8090\"}");
}

export function getRegistryInfo() {
  const url = typeof window === "undefined" ? "server side" : window.localStorage.getItem("url")
  console.log({url})
  return url
}

export function getCheckpoint() {
  const xhr = new XMLHttpRequest();
  xhr.open("GET", "http://127.0.0.1:8090/fetch/checkpoint", false);
  xhr.send(null);
  const resp = JSON.parse(xhr.response).checkpoint
  console.log({resp})
  return {
    contents: {
      logRoot: resp.contents.log_root,
      logLength: resp.contents.log_length,
      mapRoot: resp.contents.map_root
    },
    keyId: resp.key_id,
    signature: resp.signature
  };
}

export function hashCheckpoint(checkpoint) {
  const preview = enc.encode("WARG-MAP-CHECKPOINT-V0")
  console.log({checkpoint})
  const length = new Uint8Array([checkpoint.contents.log_length])
  const logRootLength = new Uint8Array([checkpoint.contents.log_root.length])
  const logRoot = enc.encode(checkpoint.contents.log_root)
  const mapRootLength = new Uint8Array([checkpoint.contents.map_root.length])
  const mapRoot = enc.encode(checkpoint.contents.map_root)
  const total = preview.length + length.length + logRoot.length + logRootLength.length + mapRoot.length + mapRootLength.length
  const all = new Uint8Array(total)
  console.log("HASHING CHECKPOINT")
  all.set(preview)
  all.set(length, preview.length)
  all.set(logRootLength, preview.length + length.length)
  all.set(logRoot, preview.length + length.length + logRootLength.length)
  all.set(mapRootLength, preview.length + length.length + logRoot.length + logRootLength.length)
  all.set(mapRoot, preview.length + length.length + logRoot.length + logRootLength.length + mapRootLength.length)
  return all
  // const val = dec.decode(all)
  // console.log({val})
  // await crypto.subtle.digest("SHA-256", all).then(dig => {
  //   console.log({dig})
  //   let temp = Array.from(new Uint8Array(dig))
  //   let fin = temp.map((b) => b.toString(16).padStart(2, "0")).join("")
  //   console.log({fin})
  //   return fin
  // })
  // const worker = new Worker("web-worker.js")
  // let root
  // worker.addEventListener("message", (e) => {
  //   console.log("main thread", {e})
  //   root = e.data
  // })
  // worker.postMessage({ type: "makeRequest", data: all })
  // console.log({root})
  // return root
  // console.log({root})
  // const resp = await fetch("/hash/checkpoint")
  // const xhr = new XMLHttpRequest()
  // xhr.open("GET", "/hash/checkpoint", false);
  // xhr.send(null);
  // const resp = xhr.responseText
  // console.log({resp})
  

}

export function fetchLogsFromLatestCheckpoint() {
  const checkpoint = getCheckpoint()
  const root = hashCheckpoint(checkpoint)
  console.log({root})
}